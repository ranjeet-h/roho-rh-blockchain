# ROHO Operational To-Do List

This guide details the exact steps to execute Phase 5 (Shadow Network) and Phase 6 (Mainnet Launch).

## Prerequisites

- Terminal/Command Prompt
- `cargo` (Rust) installed
- **One or more computers** (Instruction below covers single and multi-machine setups)

---

## ✅ Phase 5: Shadow Network (Private Test)

**Goal**: Run the software to ensure it doesn't crash and mines blocks.

### Step 1: Build the Release Binary

Run this on ALL machines you plan to use.

**macOS / Linux**:

```bash
cd rh-core
cargo clean
cargo build --release --locked
```

**Windows (PowerShell)**:

```powershell
cd rh-core
cargo clean
cargo build --release --locked
```

_Result_: You will have a binary at `target/release/rh-node` (macOS/Linux) or `target\release\rh-node.exe` (Windows).

### Step 2: Verify Binary Hash

Confirm all machines are running the exact same code.

**macOS / Linux**:

```bash
shasum -a 256 target/release/rh-node
```

**Windows (PowerShell)**:

```powershell
certutil -hashfile target\release\rh-node.exe SHA256
```

**Expected**: `53c6319009266463149ca78c42eca44eaccb5cfa0f1daba9cbf3210912617e0d`

### Step 3: Start your node (Single Machine)

If you only have one machine, follow these steps to verify it mines blocks.

**macOS / Linux**:

```bash
./target/release/rh-node
```

**Windows (PowerShell)**:

```powershell
.\target\release\rh-node.exe
```

**What to verify**:

- Look for `⛏️  Mined block X` logs every few minutes.
- If it mines Block 1, 2, 3... your node is healthy!

---

### Step 4: Multi-Machine Network (Optional but Recommended)

To test the "Network" part (Node A talking to Node B), you need two machines.

1.  Start Node A as shown above.
2.  Start Node B on another machine.
3.  Verify they both mine independently.

_Note: If running on the SAME machine, you might get a "Address already in use" error for port 8333. For a true shadow network, use separate machines or separate folders/configs if supported._

### Step 5: Observation Period (3-7 Days)

- Leave the terminals open.
- Check back daily.
- Ensure the process hasn't crashed.
- Verify "Height" in the logs is increasing (1, 2, 3...).

### Step 6: Termination

- Press `Ctrl+C` in each terminal to stop the nodes.
- **Delete the data**: Since these blocks were for testing, they are now "shadow" history. The real chain hasn't started yet.

---

## 🚀 Phase 6: Mainnet Launch (The Real Thing)

**Goal**: Release the software to the world.

### Step 1: Final Clean Build

Ensure no test artifacts remain.

```bash
cargo clean
cargo build --release --locked
```

### Step 2: Push to GitHub

1. Create a public repository (e.g., `github.com/yourname/roho`).
2. Push the code including the `roho-v1.1-fixed` tag.

```bash
git remote add origin https://github.com/yourname/roho.git
git push -u origin master
git push --tags
```

### Step 3: Publish the Release

1. Go to your GitHub Repo -> "Releases".
2. Draft a new release.
3. Choose tag: `roho-v1.1-fixed`.
4. Title: "ROHO v1.1 - Genesis".
5. Description: Paste the contents of `RELEASE_NOTES.txt`.
6. Upload the `rh-node` binary (optional, users should build from source).

### Step 4: Start the First Seed Node

Run the node on a machine that will stay online (e.g., a VPS/Server).

```bash
./target/release/rh-node
```

**CONGRATULATIONS.**
The first block mined by this specific process is Block #1 of the ROHO Mainnet.
You are now live.
