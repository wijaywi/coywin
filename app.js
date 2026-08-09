document.addEventListener('DOMContentLoaded', () => {
    fetchImages();
    // Auto-sync every 10 seconds
    setInterval(fetchImages, 10000);
});

let knownImages = new Set();
let isFirstLoad = true;

async function fetchImages() {
    try {
        const response = await fetch('/api/images');
        const images = await response.json(); // Array of objects [{filename, name, hash}]
        
        document.getElementById('total-count').textContent = images.length;
        
        const gallery = document.getElementById('gallery');
        
        if (images.length === 0) {
            gallery.innerHTML = '<div class="empty-state">No art blocks have been minted yet.<br>Start your <strong>Auto Miner</strong>!</div>';
            return;
        }

        if (document.querySelector('.empty-state')) {
            gallery.innerHTML = '';
        }

        // Loop backwards so oldest is added first, prepended dynamically
        for (let i = images.length - 1; i >= 0; i--) {
            const imgData = images[i];
            
            if (!knownImages.has(imgData.filename)) {
                knownImages.add(imgData.filename);
                
                const card = document.createElement('div');
                card.className = 'art-card';
                
                if (isFirstLoad) {
                    card.style.animationDelay = `${(i % 15) * 0.08}s`;
                }
                
                let pqcBadge = '';
                if (imgData.pqc_secured) {
                    pqcBadge = `<div class="pqc-badge" title="Verified by Post-Quantum Cryptography Dilithium">🛡️ PQC Secured</div>`;
                }

                card.innerHTML = `
                    <img src="${imgData.filename}" alt="Coywin Artwork ${imgData.name}" loading="lazy">
                    ${pqcBadge}
                    <div class="artwork-title">${imgData.name}</div>
                    <div class="hash-label" title="Cryptographic Ownership ID: ${imgData.hash}">
                        Hash: ${imgData.hash.toUpperCase()}
                    </div>
                    ${imgData.pqc_secured ? `
                    <div class="miner-label" title="Miner ECDSA Public Address: ${imgData.miner_address}">
                        Miner Session ID : <strong>${imgData.miner_name}</strong>
                    </div>` : ''}
                `;
                
                gallery.insertBefore(card, gallery.firstChild);
            }
        }
        
        isFirstLoad = false;
        
    } catch (error) {
        console.error("Failed to sync with Coywin Node:", error);
    }
}
