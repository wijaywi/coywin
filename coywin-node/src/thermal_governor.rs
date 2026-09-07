use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MiningMode {
    Eco,
    Balanced,
    Performance,
    Auto,
}

impl MiningMode {
    pub fn cap_pct(&self) -> u8 {
        match self {
            MiningMode::Eco => 25,
            MiningMode::Balanced => 45,
            MiningMode::Performance => 65,
            // Auto = hard safety ceiling, identik dengan tabel asli (<55->50%, dst).
            // Keputusan desain: Auto cap=50% dikonfirmasi sebagai hard safety ceiling;
            // Eco/Balanced/Performance adalah pilihan eksplisit pengguna yang lebih
            // agresif dari default Auto.
            MiningMode::Auto => 50,
        }
    }
}

// Asumsi threading yang disengaja:
//   - tick() dipanggil dari satu thread governor secara serial.
//   - current_duty_pct hanya DIBACA (tidak ditulis) dari thread lain (misal UI/mining loop).
//   - Dengan asumsi ini Ordering::Relaxed cukup: tidak ada kebutuhan
//     happens-before antar-thread untuk nilai duty, hanya visibility eventual.
//   - paused_for_foreground_task adalah kontrak untuk CALLER (mining loop),
//     bukan untuk tick() governor ini. Caller membaca flag ini sebelum
//     memanggil tick() dan memutuskan sendiri apakah skip tick atau tidak.
//     Governor tidak membaca flag ini di dalam tick() secara sengaja.
pub struct ThermalGovernor {
    pub current_duty_pct: AtomicU8,
    pub paused_for_foreground_task: AtomicBool,
    pub mode: MiningMode,
    cooldown_streak: AtomicU32,
    temp_samples: Mutex<Vec<f32>>,
}

impl ThermalGovernor {
    pub fn new(mode: MiningMode) -> Arc<Self> {
        Arc::new(Self {
            current_duty_pct: AtomicU8::new(0),
            paused_for_foreground_task: AtomicBool::new(false),
            mode,
            cooldown_streak: AtomicU32::new(0),
            temp_samples: Mutex::new(Vec::with_capacity(2)),
        })
    }

    /// Dipanggil tiap ~2 detik oleh mining loop.
    /// Mengembalikan duty cycle baru (0-cap) yang harus diaplikasikan ke thread/priority limiter.
    ///
    /// Kebijakan smoothing (keputusan desain final, lihat catatan audit):
    ///   - Hard stop (>=70C): cek RAW temp, bypass smoothing sepenuhnya. Ini satu-satunya
    ///     tempat raw dipakai langsung - keamanan mutlak tidak boleh ditunda averaging.
    ///   - Band menengah (55-70C): classify_temp = smoothed (rata-rata 2-sampel), BUKAN raw.
    ///     Trade-off sadar: noise immunity terhadap glitch sensor sesaat diprioritaskan
    ///     di atas responsivitas instan saat suhu naik. Konsekuensinya, ada lag maksimal
    ///     1 tick (~2 detik) sebelum band yang lebih ketat berlaku penuh saat suhu naik
    ///     tiba-tiba besar - diterima karena lompatan suhu besar dalam 1 tick lebih
    ///     mungkin noise sensor daripada pemanasan fisik genuine, dan band non-cold
    ///     tidak pernah menaikkan duty (hanya clip turun), jadi window recovery dari
    ///     over-restriction akibat glitch jauh lebih murah (~90 detik) daripada jika
    ///     raw dipakai langsung (~5+ menit untuk pulih dari drop tajam akibat 1 glitch).
    ///   - Pemanasan bertahap genuine (mis. 50->54->57 across tick) TIDAK kena lag berarti:
    ///     window 2-sampel sudah mengejar dalam maksimal 1-2 tick karena nilai berurutan
    ///     saling dekat.
    ///
    /// Batas ambang pakai `>=` di semua tempat (hard-stop dan band), bukan `>`:
    /// nilai batas persis (55.0, 60.0, 65.0, 70.0) SENGAJA jatuh ke sisi yang lebih
    /// membatasi/lebih ketat - konvensi standar untuk sistem keamanan termal, supaya
    /// tidak ada margin risiko tersembunyi di titik batas.
    pub fn tick(&self, raw_temp_celsius: f32) -> u8 {
        // Hard stop: raw temp, tidak perlu smoothing, keamanan tidak boleh ditunda.
        // >= (bukan >) supaya persis 70.0C juga hard-stop, bukan lolos ke band 65-70.
        // Flush buffer supaya tick-tick dingin sesudahnya langsung masuk cold band
        // tanpa "boros" tick menggeser sampel panas keluar window.
        if raw_temp_celsius >= 70.0 {
            self.temp_samples.lock().unwrap().clear();
            self.cooldown_streak.store(0, Ordering::Relaxed);
            self.current_duty_pct.store(0, Ordering::Relaxed);
            return 0;
        }

        // Smoothing 2-sample untuk meredam noise sensor - dipakai langsung sebagai
        // classify_temp, tanpa dicampur raw. Lihat catatan trade-off di docstring.
        let smoothed = {
            let mut samples = self.temp_samples.lock().unwrap();
            samples.push(raw_temp_celsius);
            if samples.len() > 2 {
                samples.remove(0);
            }
            samples.iter().sum::<f32>() / samples.len() as f32
        };

        let cap = self.mode.cap_pct() as f32;
        let current = self.current_duty_pct.load(Ordering::Relaxed);
        let classify_temp = smoothed;

        // >= di semua ambang: nilai batas persis jatuh ke band yang lebih ketat.
        let (band_target, is_cold_band): (u8, bool) = if classify_temp >= 65.0 {
            self.cooldown_streak.store(0, Ordering::Relaxed);
            ((cap * 0.20) as u8, false)
        } else if classify_temp >= 60.0 {
            self.cooldown_streak.store(0, Ordering::Relaxed);
            ((cap * 0.50) as u8, false)
        } else if classify_temp >= 55.0 {
            self.cooldown_streak.store(0, Ordering::Relaxed);
            ((cap * 0.80) as u8, false)
        } else {
            // <55C: cold band, ramp naik +5% per 30 detik sustained.
            (cap as u8, true)
        };

        let new_duty = if is_cold_band {
            // Tick interval ~2 detik -> 15 tick = 30 detik sustained.
            let streak = self.cooldown_streak.fetch_add(1, Ordering::Relaxed) + 1;
            if streak >= 15 {
                // Reset counter supaya siklus 30 detik berikutnya mulai dari 0.
                self.cooldown_streak.store(0, Ordering::Relaxed);
                current.saturating_add(5).min(band_target)
            } else {
                // Masih menunggu, tidak boleh melompat naik.
                current.min(band_target)
            }
        } else {
            // Turun boleh instan - keamanan thermal lebih penting dari gradualitas.
            // (Catatan: band non-cold tidak pernah MENAIKKAN duty, hanya clip turun -
            // ini bagian dari trade-off yang sama, lihat docstring.)
            current.min(band_target)
        };

        self.current_duty_pct.store(new_duty, Ordering::Relaxed);
        new_duty
    }
}

#[cfg(test)]
mod thermal_governor_tests {
    use super::*;

    #[test]
    fn hard_cutoff_above_70_degrees() {
        let gov = ThermalGovernor::new(MiningMode::Performance);
        for _ in 0..20 {
            gov.tick(50.0);
        }
        assert!(gov.current_duty_pct.load(Ordering::Relaxed) > 0);

        let duty = gov.tick(75.0);
        assert_eq!(duty, 0, "duty harus 0 total di atas 70 derajat C, tanpa toleransi");
    }

    #[test]
    fn hard_cutoff_at_exactly_70_degrees() {
        // Test boundary baru - memvalidasi fix >= (sebelumnya `>` membuat persis
        // 70.0C LOLOS hard-stop dan tetap jalan di band 65-70). Ini celah keamanan
        // nyata yang diperbaiki, bukan cuma kerapian test.
        let gov = ThermalGovernor::new(MiningMode::Performance);
        gov.current_duty_pct.store(gov.mode.cap_pct(), Ordering::Relaxed);

        let duty = gov.tick(70.0);
        assert_eq!(
            duty, 0,
            "persis 70.0C harus hard-stop (duty=0), bukan lolos ke band 65-70"
        );
    }

    #[test]
    fn band_boundary_at_exactly_55_degrees_falls_into_stricter_band() {
        // Test boundary baru - memvalidasi fix >=: persis 55.0C sekarang masuk
        // band 55-60 (lebih ketat), bukan cold band (paling longgar) seperti
        // versi `>` sebelumnya.
        let gov = ThermalGovernor::new(MiningMode::Eco); // cap=25
        gov.current_duty_pct.store(gov.mode.cap_pct(), Ordering::Relaxed);

        let duty = gov.tick(55.0);
        // 25% * 80% = 20.0 -> as u8 = 20
        assert_eq!(
            duty, 20,
            "persis 55.0C harus masuk band 55-60 (target 20), bukan cold band (target 25)"
        );
    }

    #[test]
    fn band_65_to_70_capped_at_20_percent_of_mode_cap() {
        let gov = ThermalGovernor::new(MiningMode::Performance); // cap=65
        gov.current_duty_pct.store(gov.mode.cap_pct(), Ordering::Relaxed);

        let duty = gov.tick(67.0);
        // 65% * 20% = 13.0 -> as u8 = 13
        assert_eq!(duty, 13, "band 65-70 harus clip duty ke tepat 13 (65 cap * 20%)");
    }

    #[test]
    fn band_60_to_65_capped_at_50_percent_of_mode_cap() {
        let gov = ThermalGovernor::new(MiningMode::Balanced); // cap=45
        gov.current_duty_pct.store(gov.mode.cap_pct(), Ordering::Relaxed);

        let duty = gov.tick(62.0);
        // 45% * 50% = 22.5 -> as u8 = 22
        assert_eq!(duty, 22, "band 60-65 harus clip duty ke tepat 22 (45 cap * 50%)");
    }

    #[test]
    fn band_55_to_60_capped_at_80_percent_of_mode_cap() {
        let gov = ThermalGovernor::new(MiningMode::Eco); // cap=25
        gov.current_duty_pct.store(gov.mode.cap_pct(), Ordering::Relaxed);

        let duty = gov.tick(57.0);
        // 25% * 80% = 20.0 -> as u8 = 20
        assert_eq!(duty, 20, "band 55-60 harus clip duty ke tepat 20 (25 cap * 80%)");
    }

    #[test]
    fn cooling_below_55_never_jumps_instantly_to_cap() {
        let gov = ThermalGovernor::new(MiningMode::Auto); // cap=50
        gov.tick(75.0);
        assert_eq!(gov.current_duty_pct.load(Ordering::Relaxed), 0);

        let duty_after_one_cold_tick = gov.tick(50.0);
        assert_eq!(
            duty_after_one_cold_tick, 0,
            "satu tick dingin saja TIDAK BOLEH langsung menaikkan duty"
        );
    }

    #[test]
    fn cooling_below_55_ramps_5_percent_per_30_seconds() {
        let gov = ThermalGovernor::new(MiningMode::Auto); // cap=50
        gov.tick(75.0);

        for _ in 0..14 {
            gov.tick(50.0);
        }
        assert_eq!(gov.current_duty_pct.load(Ordering::Relaxed), 0);

        let duty = gov.tick(50.0);
        assert_eq!(duty, 5, "setelah tepat 30 detik sustained <55C, duty harus naik +5%");

        for _ in 0..14 {
            gov.tick(50.0);
        }
        let duty2 = gov.tick(50.0);
        assert_eq!(duty2, 10);
    }

    #[test]
    fn resume_ramp_stops_exactly_at_mode_cap() {
        let gov = ThermalGovernor::new(MiningMode::Eco); // cap=25
        gov.tick(75.0);
        for _ in 0..(15 * 20) {
            gov.tick(50.0);
        }
        let duty = gov.current_duty_pct.load(Ordering::Relaxed);
        assert_eq!(
            duty, 25,
            "duty tidak boleh melebihi cap mode Eco (25%) walau sustained lama di bawah 55C"
        );
    }

    #[test]
    fn resuming_after_cutoff_resets_streak_and_stays_conservative_if_temp_fluctuates() {
        let gov = ThermalGovernor::new(MiningMode::Auto);
        gov.tick(75.0);
        for _ in 0..10 {
            gov.tick(50.0);
        }
        // classify_temp = smoothed = [50,62]/2 = 56.0 -> band 55-60, streak reset ke 0.
        gov.tick(62.0);

        for _ in 0..14 {
            gov.tick(50.0);
        }
        let duty = gov.current_duty_pct.load(Ordering::Relaxed);
        assert_eq!(
            duty, 0,
            "streak harus reset total kalau suhu sempat naik lagi, bukan melanjutkan hitungan lama"
        );
    }

    #[test]
    fn temperature_smoothing_absorbs_single_spike() {
        let gov = ThermalGovernor::new(MiningMode::Performance); // cap=65
        gov.current_duty_pct.store(gov.mode.cap_pct(), Ordering::Relaxed);
        gov.tick(50.0); // window=[50.0], steady dingin

        // Spike 68C (di bawah hard-stop 70C). smoothed=(50+68)/2=59.0 -> band 55-60
        // (bukan band 65-70/20% seandainya raw dipakai langsung). Ini pembuktian inti
        // dari keputusan desain: smoothing meredam spike sesaat secara proporsional,
        // bukan bereaksi berlebihan ke band paling ketat.
        let duty = gov.tick(68.0);
        // 65% * 80% = 52.0 -> as u8 = 52
        assert_eq!(
            duty, 52,
            "spike tunggal 68C harus diredam smoothing ke band 55-60 (52), \
             bukan langsung ke band 65-70 (13) - noise immunity adalah prioritas desain"
        );
    }

    #[test]
    fn different_modes_never_exceed_their_own_cap_even_in_coldest_band() {
        for (mode, cap) in [
            (MiningMode::Eco, 25u8),
            (MiningMode::Balanced, 45u8),
            (MiningMode::Performance, 65u8),
            (MiningMode::Auto, 50u8),
        ] {
            let gov = ThermalGovernor::new(mode);
            for _ in 0..(15 * 25) {
                gov.tick(50.0);
            }
            let duty = gov.current_duty_pct.load(Ordering::Relaxed);
            assert!(
                duty <= cap,
                "{:?}: duty {} melebihi cap {}",
                mode,
                duty,
                cap
            );
        }
    }

    #[test]
    fn rising_temperature_lag_is_an_accepted_design_tradeoff() {
        // Bukan bug - ini deklarasi eksplisit trade-off desain. classify_temp pakai
        // smoothed murni, bukan max(raw, smoothed), supaya glitch sensor sesaat tidak
        // langsung memicu over-restriction (lihat docstring tick() untuk analisis biaya
        // recovery). Konsekuensinya: kenaikan suhu mendadak besar dalam 1 tick masih
        // dibaca band lama selama 1 tick (~2 detik) sebelum window mengejar.
        let gov = ThermalGovernor::new(MiningMode::Eco); // cap=25
        gov.current_duty_pct.store(gov.mode.cap_pct(), Ordering::Relaxed);
        gov.tick(50.0); // window=[50.0], cold band, duty tetap 25

        // Raw 57C: smoothed=[50,57]/2=53.5 -> classify=53.5 -> MASIH cold band (<55)
        // untuk tick ini, duty tetap 25 walau raw sudah masuk band 55-60 (target 20).
        let duty = gov.tick(57.0);
        assert_eq!(
            duty, 25,
            "TRADE-OFF DISENGAJA: raw 57C belum meng-clip duty pada tick ini karena \
             smoothed=53.5 masih <55C. Window akan mengejar pada tick berikutnya kalau \
             suhu tetap tinggi. Kalau assert ini gagal dengan duty=20, berarti seseorang \
             mengubah classify_temp balik ke max(raw,smoothed) - itu regresi terhadap \
             keputusan noise-immunity yang sudah dikonfirmasi, bukan perbaikan."
        );
    }
}
