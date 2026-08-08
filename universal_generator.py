import hashlib
import math
from PIL import Image, ImageDraw
import sys
import json
import os

try:
    import pqcrypto.sign.ml_dsa_87 as d5
except ImportError:
    d5 = None

def get_val(h, start, length, max_val):
    """Fungsi pembantu untuk mengekstrak nilai integer dari potongan string hash."""
    sub = h[start:start+length]
    if len(sub) < length:
        # Jika indeks melebihi panjang hash, bungkus kembali (wrap-around)
        sub = (h * 2)[start:start+length]
    return int(sub, 16) % max_val

def draw_landscape(h, draw, img_width, img_height):
    bg_r, bg_g, bg_b = get_val(h, 2, 2, 256), get_val(h, 4, 2, 256), get_val(h, 6, 2, 256)
    draw.rectangle([0, 0, img_width, img_height], fill=(bg_r, bg_g, bg_b))
    
    # Matahari
    sun_rad = 15 + get_val(h, 40, 2, 30)
    sun_x = get_val(h, 42, 2, img_width)
    sun_y = 30 + get_val(h, 44, 2, 80)
    sun_col = (255, 255 - get_val(h, 46, 2, 100), 0)
    draw.ellipse([sun_x - sun_rad, sun_y - sun_rad, sun_x + sun_rad, sun_y + sun_rad], fill=sun_col)
    
    # Gambar berlapis (Gunung/Bukit)
    num_layers = 3 + get_val(h, 8, 1, 4)
    for layer in range(num_layers):
        layer_r = get_val(h, 10 + layer*6, 2, 256)
        layer_g = get_val(h, 12 + layer*6, 2, 256)
        layer_b = get_val(h, 14 + layer*6, 2, 256)
        
        freq = 1 + get_val(h, 16 + layer*2, 1, 4)
        amp = 20 + get_val(h, 18 + layer*2, 2, 60)
        y_offset = 120 + layer * 35 + get_val(h, 20 + layer*2, 2, 40)
        
        poly = [(0, img_height)]
        for x in range(0, img_width + 10, 10):
            phase = get_val(h, 22 + layer*2, 2, 100) / 100.0 * math.pi * 2
            y = y_offset + math.sin(x / img_width * math.pi * freq + phase) * amp
            poly.append((x, y))
        poly.append((img_width, img_height))
        draw.polygon(poly, fill=(layer_r, layer_g, layer_b))

    # Pohon
    num_trees = 1 + get_val(h, 48, 1, 3)
    for t in range(num_trees):
        tx = 20 + get_val(h, 50 + t*2, 2, img_width - 40)
        ty = img_height - 10 - get_val(h, 52 + t*2, 2, 40)
        tw = 10 + get_val(h, 54 + t*2, 1, 10)
        th = 30 + get_val(h, 55 + t*2, 2, 40)
        # Batang
        draw.rectangle([tx - tw//2, ty - th, tx + tw//2, ty], fill=(101, 67, 33))
        
        # Daun (Gumpalan Awan)
        leaf_r = tw * 1.8
        color = (34, 139, 34)
        # Tengah
        draw.ellipse([tx - leaf_r, ty - th - leaf_r, tx + leaf_r, ty - th + leaf_r], fill=color)
        # Kiri
        draw.ellipse([tx - leaf_r*1.5, ty - th - leaf_r*0.2, tx + leaf_r*0.2, ty - th + leaf_r*1.2], fill=color)
        # Kanan
        draw.ellipse([tx - leaf_r*0.2, ty - th - leaf_r*0.2, tx + leaf_r*1.5, ty - th + leaf_r*1.2], fill=color)
        # Atas
        draw.ellipse([tx - leaf_r*1.1, ty - th - leaf_r*1.6, tx + leaf_r*1.1, ty - th + leaf_r*0.5], fill=color)

def draw_face(h, draw, img_width, img_height):
    face_type = get_val(h, 2, 1, 5) # 0: Monyet, 1: Manusia, 2: Kucing, 3: Anjing, 4: Burung
    bg_r, bg_g, bg_b = get_val(h, 3, 2, 256), get_val(h, 5, 2, 256), get_val(h, 7, 2, 256)
    draw.rectangle([0, 0, img_width, img_height], fill=(bg_r, bg_g, bg_b))
    
    cx, cy = img_width//2, img_height//2
    head_w = 120 + get_val(h, 9, 2, 60)
    head_h = 120 + get_val(h, 11, 2, 60)
    
    face_r, face_g, face_b = get_val(h, 13, 2, 256), get_val(h, 15, 2, 256), get_val(h, 17, 2, 256)
    
    eye_dist = 20 + get_val(h, 19, 2, 30)
    eye_size = 10 + get_val(h, 21, 2, 15)
    
    is_female = get_val(h, 33, 1, 2) == 1
    
    # Bentuk Dasar Kepala dan Telinga berdasarkan Tipe
    if face_type == 4: # Burung
        draw.ellipse([cx - head_w//2, cy - head_h//2, cx + head_w//2, cy + head_h//2], fill=(face_r, face_g, face_b))
        beak_l = 30 + get_val(h, 23, 2, 40)
        draw.polygon([(cx-15, cy), (cx+15, cy), (cx, cy+beak_l)], fill='orange')
    elif face_type == 2: # Kucing
        ear_s = 40 + get_val(h, 23, 2, 30)
        draw.polygon([(cx - head_w//2, cy), (cx - head_w//2 - ear_s//2, cy - ear_s), (cx - head_w//4, cy - head_h//2)], fill=(face_r, face_g, face_b))
        draw.polygon([(cx + head_w//2, cy), (cx + head_w//2 + ear_s//2, cy - ear_s), (cx + head_w//4, cy - head_h//2)], fill=(face_r, face_g, face_b))
        draw.ellipse([cx - head_w//2, cy - head_h//2, cx + head_w//2, cy + head_h//2], fill=(face_r, face_g, face_b))
        # Kumis Kucing
        draw.line([cx-20, cy+10, cx-60, cy+5], fill='black', width=2)
        draw.line([cx-20, cy+20, cx-60, cy+25], fill='black', width=2)
        draw.line([cx+20, cy+10, cx+60, cy+5], fill='black', width=2)
        draw.line([cx+20, cy+20, cx+60, cy+25], fill='black', width=2)
    elif face_type == 3: # Anjing
        ear_w = 30 + get_val(h, 23, 2, 20)
        ear_h = 60 + get_val(h, 25, 2, 40)
        draw.ellipse([cx - head_w//2 - ear_w, cy - ear_h//2, cx - head_w//2 + ear_w, cy + ear_h], fill=(max(0, face_r-30), max(0, face_g-30), max(0, face_b-30)))
        draw.ellipse([cx + head_w//2 - ear_w, cy - ear_h//2, cx + head_w//2 + ear_w, cy + ear_h], fill=(max(0, face_r-30), max(0, face_g-30), max(0, face_b-30)))
        draw.ellipse([cx - head_w//2, cy - head_h//2, cx + head_w//2, cy + head_h//2], fill=(face_r, face_g, face_b))
        # Hidung Anjing
        draw.ellipse([cx-15, cy+10, cx+15, cy+30], fill='black')
    elif face_type == 0: # Monyet
        ear_s = 40 + get_val(h, 23, 2, 30)
        draw.ellipse([cx - head_w//2 - ear_s//2, cy - ear_s//2, cx - head_w//2 + ear_s//2, cy + ear_s//2], fill=(face_r, face_g, face_b))
        draw.ellipse([cx + head_w//2 - ear_s//2, cy - ear_s//2, cx + head_w//2 + ear_s//2, cy + ear_s//2], fill=(face_r, face_g, face_b))
        draw.ellipse([cx - head_w//2, cy - head_h//2, cx + head_w//2, cy + head_h//2], fill=(face_r, face_g, face_b))
        draw.ellipse([cx - head_w//3, cy, cx + head_w//3, cy + head_h//2], fill=(230, 200, 160)) # Moncong
    else: # Manusia
        draw.ellipse([cx - head_w//2 - 10, cy - 15, cx - head_w//2 + 10, cy + 15], fill=(face_r, face_g, face_b))
        draw.ellipse([cx + head_w//2 - 10, cy - 15, cx + head_w//2 + 10, cy + 15], fill=(face_r, face_g, face_b))
        draw.ellipse([cx - head_w//2, cy - head_h//2, cx + head_w//2, cy + head_h//2], fill=(face_r, face_g, face_b))
        # Rambut
        hair_r, hair_g, hair_b = get_val(h, 27, 2, 256), get_val(h, 29, 2, 256), get_val(h, 31, 2, 256)
        hair_style = get_val(h, 32, 1, 3)
        if is_female:
            # Rambut panjang wanita
            draw.ellipse([cx - head_w//2 - 15, cy - head_h//2 - 20, cx + head_w//2 + 15, cy + head_h//2 + 20], fill=(hair_r, hair_g, hair_b))
            draw.ellipse([cx - head_w//2, cy - head_h//2, cx + head_w//2, cy + head_h//2], fill=(face_r, face_g, face_b)) # Wajah menimpa rambut
            draw.arc([cx - head_w//2, cy - head_h//2 - 15, cx + head_w//2, cy], start=180, end=360, fill=(hair_r, hair_g, hair_b), width=20)
        else:
            # Variasi rambut pria
            if hair_style == 0:
                draw.arc([cx - head_w//2, cy - head_h//2 - 20, cx + head_w//2, cy], start=180, end=360, fill=(hair_r, hair_g, hair_b), width=25)
            elif hair_style == 1: # Kotak
                draw.rectangle([cx - head_w//2 + 10, cy - head_h//2 - 30, cx + head_w//2 - 10, cy - head_h//2], fill=(hair_r, hair_g, hair_b))
            else: # Jabrik (Spiked)
                for i in range(5):
                    draw.polygon([(cx - 40 + i*20, cy - head_h//2), (cx - 30 + i*20, cy - head_h//2 - 30), (cx - 20 + i*20, cy - head_h//2)], fill=(hair_r, hair_g, hair_b))
        
        # Hidung Manusia
        draw.polygon([(cx, cy+5), (cx-5, cy+20), (cx+5, cy+20)], fill=(max(0, face_r-40), max(0, face_g-40), max(0, face_b-40)))
        # Mulut Manusia
        mouth_w = 20 + get_val(h, 34, 2, 20)
        draw.arc([cx - mouth_w//2, cy + 25, cx + mouth_w//2, cy + 40], start=0, end=180, fill='black', width=3)
        
    # Mata (Umum)
    if face_type != 4:
        draw.ellipse([cx - eye_dist - eye_size, cy - 20 - eye_size, cx - eye_dist + eye_size, cy - 20 + eye_size], fill='white')
        draw.ellipse([cx + eye_dist - eye_size, cy - 20 - eye_size, cx + eye_dist + eye_size, cy - 20 + eye_size], fill='white')
        pupil = eye_size * 0.6
        draw.ellipse([cx - eye_dist - pupil, cy - 20 - pupil, cx - eye_dist + pupil, cy - 20 + pupil], fill='black')
        draw.ellipse([cx + eye_dist - pupil, cy - 20 - pupil, cx + eye_dist + pupil, cy - 20 + pupil], fill='black')
        # Bulu mata wanita
        if face_type == 1 and is_female:
            draw.line([(cx - eye_dist, cy - 20 - eye_size), (cx - eye_dist - 15, cy - 20 - eye_size - 10)], fill='black', width=3)
            draw.line([(cx + eye_dist, cy - 20 - eye_size), (cx + eye_dist + 15, cy - 20 - eye_size - 10)], fill='black', width=3)
            draw.line([(cx - eye_dist - 10, cy - 20 - eye_size + 2), (cx - eye_dist - 20, cy - 20 - eye_size - 5)], fill='black', width=2)
            draw.line([(cx + eye_dist + 10, cy - 20 - eye_size + 2), (cx + eye_dist + 20, cy - 20 - eye_size - 5)], fill='black', width=2)
    else: # Mata burung
        draw.ellipse([cx - eye_dist, cy - 20, cx - eye_dist + 12, cy - 8], fill='black')
        draw.ellipse([cx + eye_dist, cy - 20, cx + eye_dist + 12, cy - 8], fill='black')

def draw_abstract_geometry(h, draw, img_width, img_height):
    # This is now Space / Galaxy Theme
    bg_r, bg_g, bg_b = get_val(h, 2, 2, 40), get_val(h, 4, 2, 40), get_val(h, 6, 2, 60)
    draw.rectangle([0, 0, img_width, img_height], fill=(bg_r, bg_g, bg_b)) # Deep space
    
    # 1. Bintang (Stars)
    num_stars = 30 + get_val(h, 8, 2, 100)
    for i in range(num_stars):
        sx = get_val(h, (i*2)%60, 2, img_width)
        sy = get_val(h, (i*2+1)%60, 2, img_height)
        s_size = get_val(h, (i*3)%60, 1, 3)
        draw.ellipse([sx, sy, sx+s_size, sy+s_size], fill='white')
        
    # 2. Asteroid (Space Debris)
    num_asteroids = get_val(h, 10, 1, 8)
    for i in range(num_asteroids):
        ax = get_val(h, 11+i*2, 2, img_width)
        ay = get_val(h, 12+i*2, 2, img_height)
        a_size = 3 + get_val(h, 13+i*2, 1, 10)
        draw.polygon([(ax, ay-a_size), (ax+a_size, ay), (ax, ay+a_size), (ax-a_size, ay)], fill=(150, 150, 150))
        
    # 3. Planet-planet
    num_planets = 1 + get_val(h, 20, 1, 3)
    for i in range(num_planets):
        px = get_val(h, 21+i*4, 2, img_width)
        py = get_val(h, 23+i*4, 2, img_height)
        pr = 15 + get_val(h, 25+i*4, 2, 50) # Radius Planet
        
        pr_color = (get_val(h, 27+i*4, 2, 256), get_val(h, 29+i*4, 2, 256), get_val(h, 31+i*4, 2, 256))
        
        # Cincin Planet (30% Chance)
        has_ring = get_val(h, 33+i*4, 1, 10) > 7
        if has_ring:
            ring_w = pr * 2.5
            ring_h = pr * 0.6
            draw.ellipse([px - ring_w/2, py - ring_h/2, px + ring_w/2, py + ring_h/2], outline='white', width=3)
            
        # Bodi Planet
        draw.ellipse([px - pr, py - pr, px + pr, py + pr], fill=pr_color)
        
        # Kawah / Detail permukaan
        crater_r = pr / 3
        if crater_r > 5:
            draw.ellipse([px - crater_r, py - crater_r, px, py], fill=(max(0, pr_color[0]-50), max(0, pr_color[1]-50), max(0, pr_color[2]-50)))

def draw_wave_line_art(h, draw, img_width, img_height):
    bg_r, bg_g, bg_b = get_val(h, 2, 2, 256), get_val(h, 4, 2, 256), get_val(h, 6, 2, 256)
    draw.rectangle([0, 0, img_width, img_height], fill=(bg_r, bg_g, bg_b))
    
    num_lines = 15 + get_val(h, 8, 2, 40)
    freq1 = 1 + get_val(h, 10, 1, 6)
    freq2 = 1 + get_val(h, 11, 1, 6)
    amp_x = 20 + get_val(h, 12, 2, 80)
    amp_y = 20 + get_val(h, 14, 2, 80)
    
    line_r, line_g, line_b = get_val(h, 16, 2, 256), get_val(h, 18, 2, 256), get_val(h, 20, 2, 256)
    
    cx, cy = img_width//2, img_height//2
    
    points = []
    for t in range(0, 360, 3):
        rad = math.radians(t)
        dynamic_amp_x = amp_x + math.sin(rad * freq1) * 40
        dynamic_amp_y = amp_y + math.cos(rad * freq2) * 40
        x = cx + math.sin(rad) * dynamic_amp_x
        y = cy + math.cos(rad) * dynamic_amp_y
        points.append((x, y))
        
    # Gambar dari luar ke dalam agar bentuk terkecil menimpa yang besar
    for i in reversed(range(num_lines)):
        scale = 0.1 + (i / num_lines) * 1.5
        scaled_points = [(cx + (px - cx) * scale, cy + (py - cy) * scale) for px, py in points]
        
        # Separasi gelap-terang pada lekukan
        shade = 1.0 if i % 2 == 0 else 0.6
        cur_r = int(line_r * shade)
        cur_g = int(line_g * shade)
        cur_b = int(line_b * shade)
        
        if len(scaled_points) > 2:
            draw.polygon(scaled_points, fill=(cur_r, cur_g, cur_b))

def draw_everyday_object(h, draw, img_width, img_height):
    obj_type = get_val(h, 2, 1, 4) # 0: Mobil, 1: Sepeda, 2: Sepatu, 3: Tas
    bg_r, bg_g, bg_b = get_val(h, 3, 2, 256), get_val(h, 5, 2, 256), get_val(h, 7, 2, 256)
    draw.rectangle([0, 0, img_width, img_height], fill=(bg_r, bg_g, bg_b))
    
    cx, cy = img_width//2, img_height//2
    obj_r, obj_g, obj_b = get_val(h, 9, 2, 256), get_val(h, 11, 2, 256), get_val(h, 13, 2, 256)
    
    if obj_type == 0: # Mobil
        car_w = 120 + get_val(h, 15, 2, 60)
        car_h = 40 + get_val(h, 17, 2, 30)
        top_w = car_w * 0.6
        
        # Bodi utama
        draw.polygon([(cx - car_w//2, cy + car_h//2), (cx + car_w//2, cy + car_h//2),
                      (cx + car_w//2, cy), (cx + top_w//2, cy - car_h),
                      (cx - top_w//2, cy - car_h), (cx - car_w//2, cy)], fill=(obj_r, obj_g, obj_b))
                      
        # Jendela
        win_margin = 6
        draw.polygon([(cx, cy), (cx + top_w//2 - win_margin, cy - car_h + win_margin),
                      (cx - top_w//2 + win_margin, cy - car_h + win_margin), (cx - car_w//2 + win_margin*3, cy)], fill=(200, 230, 255))
                      
        # Bangku (terlihat dari jendela)
        seat_w = 12
        seat_h = car_h * 0.7
        draw.rectangle([cx - seat_w*1.5, cy - seat_h, cx - seat_w*0.5, cy], fill=(100, 50, 50))
        
        # Tiang pembatas jendela (Pilar B)
        draw.line([(cx, cy - car_h + win_margin), (cx, cy)], fill=(obj_r, obj_g, obj_b), width=5)
                      
        # Roda
        wheel_s = 30 + get_val(h, 19, 1, 20)
        draw.ellipse([cx - car_w//3 - wheel_s//2, cy + car_h//2 - wheel_s//2, cx - car_w//3 + wheel_s//2, cy + car_h//2 + wheel_s//2], fill='black')
        draw.ellipse([cx + car_w//3 - wheel_s//2, cy + car_h//2 - wheel_s//2, cx + car_w//3 + wheel_s//2, cy + car_h//2 + wheel_s//2], fill='black')
    elif obj_type == 1: # Sepeda
        dist = 60 + get_val(h, 15, 2, 30)
        wheel_r = 30 + get_val(h, 17, 2, 20)
        draw.ellipse([cx - dist - wheel_r, cy + 20 - wheel_r, cx - dist + wheel_r, cy + 20 + wheel_r], outline='black', width=6)
        draw.ellipse([cx + dist - wheel_r, cy + 20 - wheel_r, cx + dist + wheel_r, cy + 20 + wheel_r], outline='black', width=6)
        c_r, c_g, c_b = obj_r, obj_g, obj_b
        draw.line([cx - dist, cy + 20, cx - 10, cy + 20], fill=(c_r, c_g, c_b), width=6) 
        draw.line([cx - 10, cy + 20, cx - 30, cy - 30], fill=(c_r, c_g, c_b), width=6)
        draw.line([cx - 30, cy - 30, cx - dist, cy + 20], fill=(c_r, c_g, c_b), width=6)
        draw.line([cx - 30, cy - 30, cx + dist - 20, cy - 30], fill=(c_r, c_g, c_b), width=6)
        draw.line([cx - 10, cy + 20, cx + dist - 20, cy - 30], fill=(c_r, c_g, c_b), width=6)
        draw.line([cx + dist - 20, cy - 30, cx + dist, cy + 20], fill=(c_r, c_g, c_b), width=6)
    elif obj_type == 2: # Sepatu
        shoe_w = 120 + get_val(h, 15, 2, 50)
        shoe_h = 40 + get_val(h, 17, 2, 20)
        
        # Sol sepatu (Abu-abu, melengkung sedikit di ujung)
        draw.rectangle([cx - shoe_w//2 + 10, cy + shoe_h//2, cx + shoe_w//2 - 10, cy + shoe_h//2 + 12], fill=(80,80,80))
        draw.ellipse([cx - shoe_w//2, cy + shoe_h//2, cx - shoe_w//2 + 20, cy + shoe_h//2 + 12], fill=(80,80,80))
        draw.ellipse([cx + shoe_w//2 - 20, cy + shoe_h//2, cx + shoe_w//2, cy + shoe_h//2 + 12], fill=(80,80,80))

        # Tumit melengkung
        draw.ellipse([cx - shoe_w//2, cy - shoe_h, cx - shoe_w//2 + shoe_h*1.5, cy + shoe_h//2], fill=(obj_r, obj_g, obj_b))
        # Ujung depan melengkung (Toe box)
        draw.ellipse([cx + shoe_w//2 - shoe_h*1.5, cy - 10, cx + shoe_w//2, cy + shoe_h//2], fill=(obj_r, obj_g, obj_b))
        # Sambungan tengah
        draw.rectangle([cx - shoe_w//2 + shoe_h*0.75, cy - 10, cx + shoe_w//2 - shoe_h*0.75, cy + shoe_h//2], fill=(obj_r, obj_g, obj_b))
        
        # Leher atas sepatu (Ankle collar)
        draw.polygon([(cx - shoe_w//2 + 10, cy - shoe_h), 
                      (cx - 10, cy - shoe_h), 
                      (cx + 20, cy - 10), 
                      (cx - shoe_w//2 + shoe_h*0.75, cy - 10)], fill=(obj_r, obj_g, obj_b))
                      
        # Lidah sepatu (warna sedikit lebih gelap)
        tongue_r, tongue_g, tongue_b = max(0, obj_r-30), max(0, obj_g-30), max(0, obj_b-30)
        draw.polygon([(cx - 15, cy - shoe_h + 5), (cx + 10, cy - shoe_h - 15), (cx + 30, cy - 10), (cx, cy - 10)], fill=(tongue_r, tongue_g, tongue_b))
        
        # Tali Sepatu (Silang)
        lace_x = cx + 5
        lace_y = cy - shoe_h + 15
        for i in range(4):
            x_offset = i * 6
            y_offset = i * 8
            draw.line([lace_x + x_offset, lace_y + y_offset, lace_x + x_offset + 10, lace_y + y_offset + 6], fill='white', width=3)
            draw.line([lace_x + x_offset, lace_y + y_offset + 6, lace_x + x_offset + 10, lace_y + y_offset], fill='white', width=3)
    else: # Tas
        bag_style = get_val(h, 14, 1, 2)
        bag_w = 80 + get_val(h, 15, 2, 60)
        bag_h = 80 + get_val(h, 17, 2, 60)
        
        if bag_style == 1:
            # Tas lengkung tipis (Sling bag melengkung)
            draw.ellipse([cx - bag_w//2, cy - bag_h//3, cx + bag_w//2, cy + bag_h//2], fill=(obj_r, obj_g, obj_b), outline='black', width=3)
            # Tali melengkung panjang
            draw.arc([cx - bag_w//3, cy - bag_h, cx + bag_w//3, cy - bag_h//3 + 10], start=180, end=360, fill='black', width=5)
            # Motif saku depan
            draw.ellipse([cx - bag_w//4, cy, cx + bag_w//4, cy + bag_h//3], fill=(max(0, obj_r-30), max(0, obj_g-30), max(0, obj_b-30)))
        else:
            # Tas ransel/kotak standar
            draw.rectangle([cx - bag_w//2, cy - bag_h//2, cx + bag_w//2, cy + bag_h//2], fill=(obj_r, obj_g, obj_b), outline='black', width=4)
            draw.arc([cx - bag_w//4, cy - bag_h//2 - 40, cx + bag_w//4, cy - bag_h//2 + 10], start=180, end=360, fill='black', width=8)

import re

def generate_universal(seed_string, output_filename=None):
    if re.fullmatch(r'[a-fA-F0-9]{64}', seed_string):
        # Hash PoW selalu diawali banyak angka 0 (karena tingkat kesulitan/difficulty).
        # Jika kita membaca dari depan, semua blok akan terdeteksi sebagai kategori 0 (Pemandangan).
        # Solusi: Balik urutan karakternya (reverse) khusus untuk mesin grafis!
        h = seed_string.lower()[::-1]
    else:
        h = hashlib.sha256(seed_string.encode('utf-8')).hexdigest()
    
    # Pemecah Kategori: byte pertama modulo 5
    # Kategori: 0=Pemandangan, 1=Wajah, 2=Geometri, 3=Wave, 4=Benda
    category = get_val(h, 0, 2, 5) 
    
    img_width, img_height = 300, 300
    img = Image.new('RGB', (img_width, img_height), color='white')
    draw = ImageDraw.Draw(img)
    
    cat_names = [
        "Pemandangan Alam (Gelombang/Lengkungan)", 
        "Wajah (Manusia/Kucing/Anjing/Monyet/Burung)", 
        "Geometri Abstrak/Mandala", 
        "Wave Line Art/Alien", 
        "Benda Keseharian (Mobil/Sepeda/Sepatu/Tas)"
    ]
    
    if category == 0:
        draw_landscape(h, draw, img_width, img_height)
    elif category == 1:
        draw_face(h, draw, img_width, img_height)
    elif category == 2:
        draw_abstract_geometry(h, draw, img_width, img_height)
    elif category == 3:
        draw_wave_line_art(h, draw, img_width, img_height)
    else:
        draw_everyday_object(h, draw, img_width, img_height)

    cat_names = [
        "Landscape",
        "Character Face",
        "Space / Galaxy",
        "Wave Line Art",
        "Everyday Objects"
    ]
    
    import bip_coywin
    coywin_name = bip_coywin.hash_to_name(h)
    
    if output_filename is None:
        os.makedirs("output_images", exist_ok=True)
        output_filename = os.path.join("output_images", f"{coywin_name}_{h[:8]}.png")
        
    img.save(output_filename)
    
    # --- PQC Dilithium Integration ---
    pqc_generated = False
    if d5 is not None:
        wallet_path = r'D:\zzzzzzzzzzz AntiGravity\coiwin-node-windows\miner_wallet.json'
        if os.path.exists(wallet_path):
            try:
                with open(wallet_path, 'r') as f:
                    wallet = json.load(f)
                sk = bytes.fromhex(wallet['dilithium_secret'])
                
                # Tambahan: Ambil ECDSA (Alamat) dan jadikan nama BIP-Coywin!
                miner_address = wallet.get('ecdsa_public', 'Unknown')
                import bip_coywin
                miner_name = bip_coywin.hash_to_name(miner_address)
                
                payload = f"Coywin_Artwork|Name:{coywin_name}|Hash:{seed_string}"
                signature = d5.sign(sk, payload.encode('utf-8'))
                
                pqc_filename = output_filename.replace(".png", ".pqc")
                cert = {
                    "artwork_name": coywin_name,
                    "block_hash": seed_string,
                    "miner_address": miner_address,
                    "miner_name": miner_name,
                    "public_key": wallet['dilithium_public'],
                    "signature": signature.hex(),
                    "algorithm": "Dilithium5 (ml_dsa_87)"
                }
                with open(pqc_filename, 'w') as f:
                    json.dump(cert, f, indent=4)
                pqc_generated = True
            except Exception as e:
                print(f"    PQC Error: {e}")
                
    print(f"[+] Successfully rendered block: '{seed_string}'")
    print(f"    Name     : {coywin_name}")

    print(f"    File     : {output_filename}")
    if pqc_generated:
        print(f"    PQC      : Secured with Dilithium5\n")
    else:
        print()

if __name__ == '__main__':
    seeds = sys.argv[1:]
    if not seeds:
        print("Mendemonstrasikan Universal Generator Coywin...\n")
        demo_seeds = [
            "Lanskap Epik 1", "Lanskap Epik 2",
            "Monyet Ajaib", "Manusia Misterius 1",
            "Mandala Suci 99", "Geometri 88",
            "Gelombang Kosmik 001", "Wave Alien 22",
            "Tas Belanja Baru", "Sepeda Gunung 2026", "Mobil Balap 99"
        ]
        for s in demo_seeds:
            generate_universal(s)
    else:
        for s in seeds:
            generate_universal(s)
