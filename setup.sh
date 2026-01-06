#!/bin/bash
# Alrajhi SQL TUI - Git-based Setup
# Usage: git clone https://github.com/hszkf/alrajhi-sql-tui.git && cd alrajhi-sql-tui && ./setup.sh

set -e

echo "╔═══════════════════════════════════╗"
echo "║  🏦 ALRAJHI SQL STUDIO SETUP      ║"
echo "╚═══════════════════════════════════╝"
echo ""

# Check/Install Rust
if ! command -v cargo &> /dev/null; then
    echo "⚙️  Rust not found. Installing..."

    # Try curl first, then wget
    if command -v curl &> /dev/null; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    elif command -v wget &> /dev/null; then
        wget -qO- https://sh.rustup.rs | sh -s -- -y
    else
        echo "❌ Neither curl nor wget found."
        echo "   Please install Rust manually: https://rustup.rs"
        exit 1
    fi

    # Load Rust environment
    source "$HOME/.cargo/env"
    echo "✓ Rust installed"
else
    echo "✓ Rust found"
fi

# Build
echo "🔨 Building release (1-2 minutes)..."
cargo build --release 2>/dev/null
echo "✓ Build complete"

# Configure
if [ ! -f .env ]; then
    echo ""
    echo "⚙️  Database Configuration:"
    read -p "   Host [10.200.224.42]: " DB_HOST
    read -p "   Port [1433]: " DB_PORT
    read -p "   User [ssis_admin]: " DB_USER
    read -sp "   Password: " DB_PASSWORD
    echo ""
    read -p "   Database [Staging]: " DB_DATABASE

    cat > .env << EOF
DB_HOST=${DB_HOST:-10.200.224.42}
DB_PORT=${DB_PORT:-1433}
DB_USER=${DB_USER:-ssis_admin}
DB_PASSWORD=${DB_PASSWORD}
DB_DATABASE=${DB_DATABASE:-Staging}
EOF
    echo "✓ Config saved to .env"
fi

# Install atui command
echo ""
if [ -w /usr/local/bin ]; then
    cp atui /usr/local/bin/
    echo "✓ Installed: atui command"
else
    mkdir -p ~/bin
    cp atui ~/bin/
    echo "✓ Installed: ~/bin/atui"
    echo "   Add to PATH: export PATH=\"\$HOME/bin:\$PATH\""
fi

echo ""
echo "════════════════════════════════════"
echo "✅ DONE! Run with:"
echo ""
echo "   atui          (if in PATH)"
echo "   ./run.sh      (from this folder)"
echo "════════════════════════════════════"

# Create simple run script
cat > run.sh << 'EOF'
#!/bin/bash
cd "$(dirname "$0")"
source .env 2>/dev/null
./target/release/alrajhi_sql_tui
EOF
chmod +x run.sh
