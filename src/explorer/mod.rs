//! Embedded Web Block Explorer
//! 
//! A simple HTML-based explorer served directly by the node.

mod wallet;

pub use wallet::WALLET_HTML;

/// Explorer HTML template with embedded CSS and JavaScript
pub const EXPLORER_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>ROHO Block Explorer</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
            color: #e0e0e0;
            min-height: 100vh;
            padding: 20px;
        }
        .container { max-width: 900px; margin: 0 auto; }
        header {
            text-align: center;
            padding: 40px 0;
            border-bottom: 1px solid #333;
            margin-bottom: 30px;
        }
        h1 {
            font-size: 2.5rem;
            background: linear-gradient(90deg, #00d4ff, #7c3aed);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            margin-bottom: 10px;
        }
        .subtitle { color: #888; font-size: 1rem; }
        .stats-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }
        .stat-card {
            background: rgba(255,255,255,0.05);
            border: 1px solid rgba(255,255,255,0.1);
            border-radius: 12px;
            padding: 20px;
            text-align: center;
            backdrop-filter: blur(10px);
        }
        .stat-value {
            font-size: 2rem;
            font-weight: 700;
            color: #00d4ff;
        }
        .stat-label { font-size: 0.9rem; color: #888; margin-top: 5px; }
        .section-title {
            font-size: 1.2rem;
            color: #7c3aed;
            margin: 30px 0 15px;
            border-bottom: 1px solid #333;
            padding-bottom: 10px;
        }
        .block-list { list-style: none; }
        .block-item {
            background: rgba(255,255,255,0.03);
            border: 1px solid rgba(255,255,255,0.08);
            border-radius: 8px;
            padding: 15px;
            margin-bottom: 10px;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }
        .block-height {
            font-weight: 700;
            color: #00d4ff;
            font-size: 1.1rem;
        }
        .block-hash {
            font-family: monospace;
            font-size: 0.85rem;
            color: #888;
        }
        .loading { text-align: center; padding: 40px; color: #888; }
        .search-box {
            width: 100%;
            padding: 15px 20px;
            border: 1px solid rgba(255,255,255,0.1);
            border-radius: 8px;
            background: rgba(255,255,255,0.05);
            color: #fff;
            font-size: 1rem;
            margin-bottom: 20px;
        }
        .search-box::placeholder { color: #666; }
        .error { color: #ff6b6b; text-align: center; padding: 20px; }
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>ROHO Block Explorer</h1>
            <p class="subtitle">Immutable · Decentralized · Trustless</p>
        </header>

        <input type="text" class="search-box" id="searchInput" placeholder="Search by block hash or height...">

        <div class="stats-grid" id="stats">
            <div class="stat-card">
                <div class="stat-value" id="blockHeight">-</div>
                <div class="stat-label">Block Height</div>
            </div>
            <div class="stat-card">
                <div class="stat-value" id="totalSupply">-</div>
                <div class="stat-label">RH Issued</div>
            </div>
            <div class="stat-card">
                <div class="stat-value" id="difficulty">-</div>
                <div class="stat-label">Difficulty</div>
            </div>
            <div class="stat-card">
                <div class="stat-value" id="utxoCount">-</div>
                <div class="stat-label">UTXO Count</div>
            </div>
        </div>

        <h2 class="section-title">Recent Blocks</h2>
        <ul class="block-list" id="blockList">
            <li class="loading">Loading blocks...</li>
        </ul>
    </div>

    <script>
        const RPC_URL = window.location.origin;

        async function rpc(method, params = []) {
            const res = await fetch(RPC_URL, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ jsonrpc: '2.0', method, params, id: 1 })
            });
            const data = await res.json();
            return data.result;
        }

        async function loadStats() {
            try {
                const info = await rpc('getinfo');
                document.getElementById('blockHeight').textContent = info.blocks.toLocaleString();
                document.getElementById('totalSupply').textContent = info.total_issued.toLocaleString() + ' RH';
                document.getElementById('difficulty').textContent = (info.difficulty / 1e6).toFixed(2) + 'M';
                document.getElementById('utxoCount').textContent = info.utxo_count.toLocaleString();
            } catch (e) {
                console.error('Failed to load stats:', e);
            }
        }

        async function loadBlocks() {
            try {
                const info = await rpc('getinfo');
                const height = info.blocks;
                const blocks = [];
                
                for (let i = height; i > Math.max(0, height - 10); i--) {
                    const hash = await rpc('getblockhash', [i]);
                    if (hash) {
                        blocks.push({ height: i, hash });
                    }
                }

                const list = document.getElementById('blockList');
                list.innerHTML = blocks.map(b => `
                    <li class="block-item">
                        <span class="block-height">#${b.height}</span>
                        <span class="block-hash">${b.hash.substring(0, 24)}...</span>
                    </li>
                `).join('');
            } catch (e) {
                document.getElementById('blockList').innerHTML = 
                    '<li class="error">Failed to load blocks</li>';
            }
        }

        // Initial load
        loadStats();
        loadBlocks();

        // Auto-refresh every 10 seconds
        setInterval(() => {
            loadStats();
            loadBlocks();
        }, 10000);
    </script>
</body>
</html>
"#;
