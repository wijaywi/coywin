use std::time::{Duration, Instant};

/// Mengkonversi `duty_pct` dari ThermalGovernor menjadi work/sleep duration
/// per periode duty cycle.
///
/// Desain: time-budget pattern.
/// Mining loop memanggil `begin_period()` di awal setiap periode untuk mendapat
/// `WorkBudget`, menjalankan batch hash sampai `WorkBudget::is_exhausted()`,
/// lalu memanggil `WorkBudget::sleep_remainder()` untuk tidur sisa periode.
/// Limiter tidak perlu tahu berapa lama satu batch hash - mining loop yang
/// memutuskan kapan cek budget (tiap batch, tiap N batch, dst).
///
/// CONSTRAINT KRITIS untuk mining loop (lihat juga `WorkBudget::is_exhausted`):
/// Enforcement duty rendah bergantung sepenuhnya pada `is_exhausted()` dicek
/// di antara batch. Presisi throttle dibatasi oleh durasi satu unit kerja
/// terkecil yang bisa di-checkpoint. Kalau satu batch terlalu panjang
/// relatif terhadap work_budget, loop tetap jalan melewati budget tanpa error.
///
/// Mengapa period=500ms:
/// - Windows default timer resolution ~15.6ms.
/// - Duty terendah realistis: 5% (lantai ramp semua mode - increment pertama
///   pasca hard-stop adalah +5%, berlaku untuk Eco/Balanced/Performance/Auto).
///   work_budget = 500 * 5/100 = 25ms.
///   Jitter 15.6ms / 25ms work = ~62% error di fase ini.
///   Ini bukan window acak - duty=5% dipertahankan selama 30 detik penuh
///   (15 tick) pasca hard-stop, tepat saat termal paling sensitif.
/// - Angka 13% (band 65-70, Performance) bukan worst-case - hanya contoh band.
///   Work=65ms, jitter~24% - jauh lebih baik dari duty=5%.
/// - Argumen yang tetap valid untuk 500ms: governor ramp paling cepat +5%/30s,
///   tidak ada kebutuhan responsivitas sub-detik untuk period itu sendiri.
///   Tapi implikasi 62% jitter di duty=5% perlu terdokumentasi eksplisit
///   supaya tidak diwarisi sebagai asumsi tersembunyi.
/// - `timeBeginPeriod(1)` TIDAK dipanggil: meningkatkan timer resolution
///   sistem-wide kontradiksi dengan desain low-power background miner.
///   Jika profiling hardware menunjukkan jitter tidak bisa diterima, keputusan
///   ini bisa dibalik dengan data nyata (bukan asumsi).
pub struct ThreadLimiter {
    period: Duration,
}

impl ThreadLimiter {
    pub fn new(period_ms: u64) -> Self {
        Self {
            period: Duration::from_millis(period_ms),
        }
    }

    /// Memulai periode baru. Mengembalikan `WorkBudget` yang mining loop pakai
    /// untuk menentukan kapan berhenti kerja dan mulai tidur.
    /// duty_pct=0 menghasilkan budget nol (full sleep period).
    /// duty_pct>=100 diklem ke 100 (full work period, sleep=0).
    pub fn begin_period(&self, duty_pct: u8) -> WorkBudget {
        let clamped = duty_pct.min(100) as u64;
        let work_ms = self.period.as_millis() as u64 * clamped / 100;
        WorkBudget {
            period: self.period,
            work_budget: Duration::from_millis(work_ms),
            period_start: Instant::now(),
        }
    }
}

/// Handle untuk satu periode duty cycle.
/// Dibuat oleh `ThreadLimiter::begin_period()`, dipakai oleh mining loop.
pub struct WorkBudget {
    period: Duration,
    work_budget: Duration,
    period_start: Instant,
}

impl WorkBudget {
    /// Apakah budget kerja untuk periode ini sudah habis?
    /// Mining loop memanggil ini di antara batch.
    ///
    /// KONTRAK UNTUK MINING LOOP (directive untuk fase integrasi):
    /// Durasi satu unit kerja terkecil yang di-checkpoint HARUS jauh di bawah
    /// work_budget duty terendah yang governor hasilkan. Nilai referensi:
    ///   - duty=5% (lantai ramp pasca hard-stop): work_budget = 25ms di period 500ms
    ///   - duty=13% (band 65-70, Performance): work_budget = 65ms
    ///
    /// Pertanyaan yang harus dijawab saat profiling hardware:
    ///   "Apakah satu unit kerja terkecil yang bisa di-checkpoint < 25ms?"
    ///
    /// Kalau tidak (mis. satu batch hash makan 100-300ms), mining loop WAJIB
    /// memecah pengecekan `is_exhausted()` ke granularitas sub-batch (misal
    /// tiap N hash, bukan tiap batch penuh). Tanpa itu, duty=5% yang diminta
    /// governor bisa terealisasi sebagai 60%+ di hardware, tepat di kondisi
    /// paling kritis secara termal (30 detik pertama pasca hard-stop).
    pub fn is_exhausted(&self) -> bool {
        self.period_start.elapsed() >= self.work_budget
    }

    /// Tidur sisa periode. Dipanggil setelah `is_exhausted()` true.
    /// Jika periode sudah lewat (mis. batch terakhir makan waktu lama),
    /// fungsi ini langsung return tanpa sleep - tidak pernah blok negatif.
    pub fn sleep_remainder(self) {
        let elapsed = self.period_start.elapsed();
        if elapsed < self.period {
            std::thread::sleep(self.period - elapsed);
        }
    }

    /// Untuk keperluan test: berapa ms work budget yang dialokasikan?
    pub fn work_budget_ms(&self) -> u64 {
        self.work_budget.as_millis() as u64
    }

    /// Untuk keperluan test: berapa ms sleep yang tersisa jika dipanggil sekarang?
    /// Nilai ini berkurang seiring waktu (bukan fixed). Pakai dengan bijak di test.
    pub fn sleep_remainder_ms_approx(&self) -> u64 {
        let elapsed = self.period_start.elapsed();
        if elapsed < self.period {
            (self.period - elapsed).as_millis() as u64
        } else {
            0
        }
    }
}

#[cfg(test)]
mod thread_limiter_tests {
    use super::*;

    // Semua test di sini TIDAK mengukur waktu nyata (tidak ada thread::sleep).
    // Yang diverifikasi: aritmetika work/sleep budget, batas, dan kontrak interface.
    // Verifikasi CPU% riil (apakah duty=30% benar-benar ~30% CPU usage di mesin target)
    // adalah langkah terpisah yang butuh pengukuran manual dengan hardware.

    fn limiter_500ms() -> ThreadLimiter {
        ThreadLimiter::new(500)
    }

    #[test]
    fn work_budget_proportional_to_duty_pct() {
        let lim = limiter_500ms();
        // 500ms * 50% = 250ms work
        assert_eq!(lim.begin_period(50).work_budget_ms(), 250);
    }

    #[test]
    fn duty_0_gives_zero_work_budget() {
        let lim = limiter_500ms();
        let budget = lim.begin_period(0);
        assert_eq!(budget.work_budget_ms(), 0);
        // Langsung exhausted setelah dibuat (elapsed >= 0ms work budget).
        assert!(budget.is_exhausted(), "duty=0 harus langsung exhausted, tidak ada kerja");
    }

    #[test]
    fn duty_100_gives_full_period_as_work_budget() {
        let lim = limiter_500ms();
        assert_eq!(lim.begin_period(100).work_budget_ms(), 500);
    }

    #[test]
    fn duty_above_100_clamped_to_100() {
        let lim = limiter_500ms();
        // duty_pct=200 harus diklem ke 100, bukan menghasilkan 1000ms
        assert_eq!(lim.begin_period(200).work_budget_ms(), 500);
    }

    #[test]
    fn work_budget_rounds_down_not_up() {
        let lim = limiter_500ms();
        // 500ms * 13% = 65ms (integer division: 500*13/100=65, tidak ada sisa)
        assert_eq!(lim.begin_period(13).work_budget_ms(), 65);
        // 500ms * 22% = 110ms
        assert_eq!(lim.begin_period(22).work_budget_ms(), 110);
        // 500ms * 45% = 225ms
        assert_eq!(lim.begin_period(45).work_budget_ms(), 225);
        // 500ms * 52% = 260ms
        assert_eq!(lim.begin_period(52).work_budget_ms(), 260);
    }

    #[test]
    fn governor_cap_values_produce_correct_budgets() {
        // Verifikasi semua nilai cap yang governor hasilkan (25, 45, 50, 65)
        // dan band targets (20%, 50%, 80% dari masing-masing cap) menghasilkan
        // budget yang masuk akal - tidak ada overflow atau pembulatan aneh.
        let lim = limiter_500ms();
        // Lantai ramp semua mode: duty=5% -> work=25ms (worst-case jitter)
        assert_eq!(lim.begin_period(5).work_budget_ms(), 25);
        // Eco cap=25: band targets 20, 25
        assert_eq!(lim.begin_period(20).work_budget_ms(), 100);
        assert_eq!(lim.begin_period(25).work_budget_ms(), 125);
        // Balanced cap=45: band targets 22, 36, 45
        assert_eq!(lim.begin_period(22).work_budget_ms(), 110);
        assert_eq!(lim.begin_period(36).work_budget_ms(), 180); // 45*80%=36
        assert_eq!(lim.begin_period(45).work_budget_ms(), 225);
        // Auto cap=50: band targets 10, 25, 40, 50
        assert_eq!(lim.begin_period(10).work_budget_ms(), 50);  // 50*20%=10
        assert_eq!(lim.begin_period(25).work_budget_ms(), 125); // 50*50%=25
        assert_eq!(lim.begin_period(40).work_budget_ms(), 200); // 50*80%=40
        assert_eq!(lim.begin_period(50).work_budget_ms(), 250);
        // Performance cap=65: band targets 13, 32, 52, 65
        assert_eq!(lim.begin_period(13).work_budget_ms(), 65);
        assert_eq!(lim.begin_period(32).work_budget_ms(), 160); // 65*50%=32.5->32
        assert_eq!(lim.begin_period(52).work_budget_ms(), 260); // 65*80%=52
        assert_eq!(lim.begin_period(65).work_budget_ms(), 325);
    }

    #[test]
    fn period_size_independence() {
        // Kontrak work_budget harus berlaku untuk berbagai ukuran period.
        for &period_ms in &[100u64, 250, 500, 1000] {
            let lim = ThreadLimiter::new(period_ms);
            let budget = lim.begin_period(50);
            assert_eq!(budget.work_budget_ms(), period_ms / 2,
                "period={}ms, duty=50%: work harus {}", period_ms, period_ms / 2);
        }
    }
}
