rustup update
# rustup default nightly
rustup component add clippy-preview rustfmt

sed -i 's/plugins=(/&history-substring-search /' /home/vscode/.zshrc

rustc --version
