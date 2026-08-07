import os
import json
import http.server
import socketserver

PORT = 8080

class CoywinHandler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path == '/api/images':
            self.send_response(200)
            self.send_header('Content-type', 'application/json')
            self.send_header('Cache-Control', 'no-cache, no-store, must-revalidate')
            self.end_headers()
            
            images = []
            for f in os.listdir('.'):
                if f.endswith('.png') and '_' in f:
                    stat = os.stat(f)
                    # Expected format: Name_hashpart.png (e.g. Fihekilfong_8a7b6c5d.png)
                    parts = f.replace('.png', '').split('_')
                    if len(parts) >= 2:
                        name = parts[0]
                        hash_part = parts[-1]
                        
                        pqc_file = f.replace('.png', '.pqc')
                        pqc_secured = os.path.exists(pqc_file)
                        
                        miner_name = "Unknown"
                        miner_address = "Unknown"
                        if pqc_secured:
                            try:
                                with open(pqc_file, 'r') as pf:
                                    cert_data = json.load(pf)
                                    miner_name = cert_data.get('miner_name', 'Unknown')
                                    miner_address = cert_data.get('miner_address', 'Unknown')
                            except Exception:
                                pass
                        
                        images.append({
                            'filename': f,
                            'name': name,
                            'hash': hash_part,
                            'time': stat.st_ctime,
                            'pqc_secured': pqc_secured,
                            'miner_name': miner_name,
                            'miner_address': miner_address
                        })
                    
            images.sort(key=lambda x: x['time'], reverse=True)
            self.wfile.write(json.dumps(images).encode())
            return
            
        return super().do_GET()

print("="*50)
print(f" COYWIN VIEWER SERVER RUNNING ")
print("="*50)
print(f"Please open your browser and visit:")
print(f"👉 http://localhost:{PORT}")
print("This server will automatically refresh the gallery")
print("when a new artwork block is minted!")
print("Press Ctrl+C to shut down the server.")
print("-" * 50)

socketserver.TCPServer.allow_reuse_address = True
with socketserver.TCPServer(("", PORT), CoywinHandler) as httpd:
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\nShutting down server...")
