//! Web Wallet Interface
//! 
//! Simple HTML wallet for managing RH coins.

/// Wallet HTML with key management and transaction support
pub const WALLET_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>ROHO Wallet</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(135deg, #0f0c29 0%, #302b63 50%, #24243e 100%);
            color: #e0e0e0;
            min-height: 100vh;
            padding: 20px;
        }
        .container { max-width: 600px; margin: 0 auto; }
        header {
            text-align: center;
            padding: 30px 0;
            margin-bottom: 30px;
        }
        h1 {
            font-size: 2rem;
            background: linear-gradient(90deg, #f093fb, #f5576c);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            margin-bottom: 5px;
        }
        .subtitle { color: #888; font-size: 0.9rem; }
        .card {
            background: rgba(255,255,255,0.05);
            border: 1px solid rgba(255,255,255,0.1);
            border-radius: 16px;
            padding: 25px;
            margin-bottom: 20px;
            backdrop-filter: blur(10px);
        }
        .card-title {
            font-size: 0.85rem;
            color: #888;
            text-transform: uppercase;
            letter-spacing: 1px;
            margin-bottom: 15px;
        }
        .balance {
            font-size: 2.5rem;
            font-weight: 700;
            color: #f5576c;
        }
        .balance-unit { font-size: 1rem; color: #888; }
        .address-box {
            background: rgba(0,0,0,0.3);
            border-radius: 8px;
            padding: 12px;
            font-family: monospace;
            font-size: 0.85rem;
            word-break: break-all;
            color: #f093fb;
            margin-top: 10px;
        }
        .btn {
            width: 100%;
            padding: 15px;
            border: none;
            border-radius: 8px;
            font-size: 1rem;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.3s ease;
            margin-top: 10px;
        }
        .btn-primary {
            background: linear-gradient(90deg, #f093fb, #f5576c);
            color: white;
        }
        .btn-primary:hover { transform: translateY(-2px); box-shadow: 0 5px 20px rgba(245,87,108,0.4); }
        .btn-secondary {
            background: rgba(255,255,255,0.1);
            color: #e0e0e0;
            border: 1px solid rgba(255,255,255,0.2);
        }
        .btn-secondary:hover { background: rgba(255,255,255,0.15); }
        input, textarea {
            width: 100%;
            padding: 12px 15px;
            border: 1px solid rgba(255,255,255,0.1);
            border-radius: 8px;
            background: rgba(0,0,0,0.3);
            color: #fff;
            font-size: 1rem;
            margin-bottom: 10px;
        }
        input::placeholder, textarea::placeholder { color: #666; }
        label { display: block; color: #888; font-size: 0.85rem; margin-bottom: 5px; }
        .hidden { display: none; }
        .status {
            text-align: center;
            padding: 15px;
            border-radius: 8px;
            margin-top: 15px;
        }
        .status-success { background: rgba(0,255,136,0.1); color: #00ff88; }
        .status-error { background: rgba(255,0,0,0.1); color: #ff6b6b; }
        .tabs {
            display: flex;
            gap: 10px;
            margin-bottom: 20px;
        }
        .tab {
            flex: 1;
            padding: 12px;
            text-align: center;
            background: rgba(255,255,255,0.05);
            border: 1px solid rgba(255,255,255,0.1);
            border-radius: 8px;
            cursor: pointer;
            transition: all 0.3s ease;
        }
        .tab.active {
            background: linear-gradient(90deg, #f093fb, #f5576c);
            border-color: transparent;
        }
        .tab:hover:not(.active) { background: rgba(255,255,255,0.1); }
        .warning {
            background: rgba(255,193,7,0.1);
            border: 1px solid rgba(255,193,7,0.3);
            color: #ffc107;
            padding: 12px;
            border-radius: 8px;
            font-size: 0.85rem;
            margin-bottom: 15px;
        }
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>💎 ROHO Wallet</h1>
            <p class="subtitle">Secure · Non-Custodial · Local</p>
        </header>

        <div class="tabs">
            <div class="tab active" onclick="showTab('wallet', this)">Wallet</div>
            <div class="tab" onclick="showTab('send', this)">Send</div>
            <div class="tab" onclick="showTab('keys', this)">Keys</div>
        </div>

        <!-- Wallet Tab -->
        <div id="tab-wallet">
            <div class="card">
                <div class="card-title">Your Balance</div>
                <div class="balance" id="balance">0.00 <span class="balance-unit">RH</span></div>
            </div>

            <div class="card">
                <div class="card-title">Your Address</div>
                <div class="address-box" id="address">No wallet loaded</div>
                <button class="btn btn-secondary" onclick="copyAddress()">📋 Copy Address</button>
            </div>

            <button class="btn btn-primary" onclick="refreshBalance()">🔄 Refresh Balance</button>
        </div>

        <!-- Send Tab -->
        <div id="tab-send" class="hidden">
            <div class="card">
                <div class="card-title">Send RH</div>
                <label>Recipient Address</label>
                <input type="text" id="sendTo" placeholder="RH...">
                <label>Amount (RH)</label>
                <input type="number" id="sendAmount" placeholder="0.00" step="0.00000001">
                <button class="btn btn-primary" onclick="sendTransaction()">📤 Send Transaction</button>
                <div id="sendStatus"></div>
            </div>
        </div>

        <!-- Keys Tab -->
        <div id="tab-keys" class="hidden">
            <div class="card">
                <div class="card-title">Generate New Wallet</div>
                <div class="warning">⚠️ This will replace your current wallet. Make sure to backup your private key first!</div>
                <button class="btn btn-primary" onclick="generateWallet()">🔑 Generate New Wallet</button>
            </div>

            <div class="card">
                <div class="card-title">Import Private Key</div>
                <textarea id="importKey" rows="2" placeholder="Enter your private key (hex)..."></textarea>
                <button class="btn btn-secondary" onclick="importWallet()">📥 Import Wallet</button>
            </div>

            <div class="card">
                <div class="card-title">Export Private Key</div>
                <div class="warning">⚠️ Never share your private key with anyone!</div>
                <button class="btn btn-secondary" onclick="exportKey()">📤 Show Private Key</button>
                <div class="address-box hidden" id="privateKeyDisplay"></div>
            </div>
            <div id="keysStatus"></div>
        </div>
    </div>

    <script>
        const RPC_URL = window.location.origin;
        let currentAddress = localStorage.getItem('roho_address') || null;
        let currentPrivateKey = localStorage.getItem('roho_privkey') || null;

        async function rpc(method, params = []) {
            const res = await fetch(RPC_URL, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ jsonrpc: '2.0', method, params, id: 1 })
            });
            const data = await res.json();
            if (data.error) throw new Error(data.error.message);
            return data.result;
        }

        function showTab(tabName, el) {
            document.querySelectorAll('[id^="tab-"]').forEach(t => t.classList.add('hidden'));
            document.getElementById('tab-' + tabName).classList.remove('hidden');
            document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
            if (el) el.classList.add('active');
        }

        async function refreshBalance() {
            if (!currentAddress) {
                document.getElementById('balance').textContent = '0.00 RH';
                return;
            }
            try {
                const balance = await rpc('getbalance', [currentAddress]);
                document.getElementById('balance').innerHTML = 
                    balance.toLocaleString(undefined, {minimumFractionDigits: 2, maximumFractionDigits: 8}) + 
                    ' <span class="balance-unit">RH</span>';
            } catch (e) {
                console.error('Failed to get balance:', e);
            }
        }

        async function generateWallet() {
            const status = document.getElementById('keysStatus');
            try {
                const result = await rpc('getnewaddress');
                currentAddress = result.address;
                currentPrivateKey = result.private_key;
                
                localStorage.setItem('roho_address', currentAddress);
                localStorage.setItem('roho_privkey', currentPrivateKey);
                
                document.getElementById('address').textContent = currentAddress;
                refreshBalance();
                
                status.className = 'status status-success';
                status.textContent = 'New wallet generated successfully!';
                console.log('Generated new wallet:', currentAddress);
                
                setTimeout(() => { status.textContent = ''; status.className = ''; }, 3000);
            } catch (e) {
                console.error('Failed to generate wallet:', e);
                status.className = 'status status-error';
                status.textContent = 'Error generating wallet: ' + e;
            }
        }

        async function importWallet() {
            const privKey = document.getElementById('importKey').value.trim();
            const status = document.getElementById('keysStatus');
            if (!privKey) return;
            
            try {
                console.log('Attempting to import private key...');
                const result = await rpc('importprivkey', [privKey]);
                currentAddress = result.address;
                currentPrivateKey = privKey;
                
                localStorage.setItem('roho_address', currentAddress);
                localStorage.setItem('roho_privkey', currentPrivateKey);
                
                document.getElementById('address').textContent = currentAddress;
                refreshBalance();
                
                status.className = 'status status-success';
                status.textContent = 'Wallet imported successfully!';
                console.log('Imported wallet address:', currentAddress);
                
                // Switch back to wallet tab after a short delay
                setTimeout(() => {
                    showTab('wallet', document.querySelector('.tab'));
                    status.textContent = '';
                    status.className = '';
                }, 1500);
            } catch (e) {
                console.error('Failed to import wallet:', e);
                status.className = 'status status-error';
                status.textContent = 'Invalid private key: ' + e;
            }
        }

        function exportKey() {
            const display = document.getElementById('privateKeyDisplay');
            if (currentPrivateKey) {
                display.textContent = currentPrivateKey;
                display.classList.remove('hidden');
            } else {
                alert('No private key available.');
            }
        }

        function copyAddress() {
            if (currentAddress) {
                navigator.clipboard.writeText(currentAddress);
                alert('Address copied!');
            }
        }

        async function sendTransaction() {
            const to = document.getElementById('sendTo').value.trim();
            const amount = parseFloat(document.getElementById('sendAmount').value);
            const status = document.getElementById('sendStatus');
            
            if (!to || isNaN(amount) || amount <= 0) {
                status.className = 'status status-error';
                status.textContent = 'Please enter a valid address and amount.';
                return;
            }

            status.textContent = 'Creating transaction...';
            status.className = 'status';

            try {
                // 1. Create raw transaction (passing source address to ensure we only spend our own coins)
                const unsignedHex = await rpc('createrawtransaction', [to, amount, currentAddress]);
                
                // 2. Sign raw transaction
                if (!currentPrivateKey) {
                    status.textContent = 'Private key not found. Go to Keys tab to import/generate.';
                    status.className = 'status status-error';
                    return;
                }

                status.textContent = 'Signing transaction...';
                const signedHex = await rpc('signrawtransaction', [unsignedHex, currentPrivateKey]);

                // 3. Send raw transaction
                status.textContent = 'Broadcasting transaction...';
                const txid = await rpc('sendrawtransaction', [signedHex]);

                status.textContent = 'Success! TXID: ' + txid;
                status.className = 'status status-success';
                
                // Refresh balance after a short delay
                setTimeout(refreshBalance, 3000);
            } catch (e) {
                status.textContent = 'Error: ' + (e.message || e);
                status.className = 'status status-error';
            }
        }

        // Initialize
        async function init() {
            // Clean up old placeholder keys
            if (currentPrivateKey === 'generated-on-server') {
                localStorage.removeItem('roho_privkey');
                currentPrivateKey = null;
            }

            // Sync identity: If we have a local key, "push" it to the node
            if (currentPrivateKey) {
                try {
                    console.log('Syncing identity to node...');
                    const result = await rpc('importprivkey', [currentPrivateKey]);
                    currentAddress = result.address;
                    localStorage.setItem('roho_address', currentAddress);
                } catch (e) {
                    console.error('Failed to sync identity to node:', e);
                }
            } 
            // Otherwise, if missing identity, "pull" from node (miner sync)
            else if (!currentAddress) {
                try {
                    console.log('Fetching identity from miner...');
                    const minerData = await rpc('getmineraddress');
                    currentAddress = minerData.address || minerData;
                    currentPrivateKey = minerData.private_key || currentPrivateKey;
                    
                    if (currentAddress) localStorage.setItem('roho_address', currentAddress);
                    if (currentPrivateKey) localStorage.setItem('roho_privkey', currentPrivateKey);
                } catch (e) {
                    console.error('Failed to sync miner data:', e);
                }
            }

            // Update UI
            document.getElementById('address').textContent = currentAddress || 'No Address';
            if (currentAddress) refreshBalance();
        }
        
        init();
    </script>
</body>
"#;
