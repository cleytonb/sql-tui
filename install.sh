#!/bin/bash
# Alrajhi SQL Studio - One-liner Installer
# Usage: curl -fsSL https://raw.githubusercontent.com/hszkf/alrajhi-sql-tui/master/install.sh | bash

set -e

echo "🏦 Alrajhi SQL Studio Installer"
echo "================================"

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust not found. Installing..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# Clone or update
INSTALL_DIR="$HOME/alrajhi-sql-tui"
if [ -d "$INSTALL_DIR" ]; then
    echo "📦 Updating existing installation..."
    cd "$INSTALL_DIR"
    git pull
else
    echo "📦 Cloning repository..."
    git clone https://github.com/hszkf/alrajhi-sql-tui.git "$INSTALL_DIR"
    cd "$INSTALL_DIR"
fi

# Build
echo "🔨 Building (this may take a minute)..."
cargo build --release 2>/dev/null

# Create .env if not exists
if [ ! -f .env ]; then
    echo ""
    echo "⚙️  Configure database connection:"
    read -p "   DB Host [10.200.224.42]: " DB_HOST
    read -p "   DB Port [1433]: " DB_PORT
    read -p "   DB User [ssis_admin]: " DB_USER
    read -sp "   DB Password: " DB_PASSWORD
    echo ""
    read -p "   DB Database [Staging]: " DB_DATABASE

    cat > .env << EOF
DB_HOST=${DB_HOST:-10.200.224.42}
DB_PORT=${DB_PORT:-1433}
DB_USER=${DB_USER:-ssis_admin}
DB_PASSWORD=${DB_PASSWORD}
DB_DATABASE=${DB_DATABASE:-Staging}
EOF
    echo "✅ Configuration saved to .env"
fi

# Create launcher script
cat > "$HOME/sql-studio" << 'EOF'
#!/bin/bash
cd "$HOME/alrajhi-sql-tui"
source .env 2>/dev/null
./target/release/alrajhi_sql_tui "$@"
EOF
chmod +x "$HOME/sql-studio"

echo ""
echo "✅ Installation complete!"
echo ""
echo "   Run with:  ~/sql-studio"
echo "   Or:        cd ~/alrajhi-sql-tui && source .env && ./target/release/alrajhi_sql_tui"
echo ""
