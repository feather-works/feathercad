#!/bin/sh
# Monta feathercad-linux-x86_64.tar.gz a partir do .deb do Tauri: é o pacote que o
# FeatherWorks Launcher baixa no Linux ("<app>-<versão>/instalar.sh", instala em ~/.local).
# Uso: scripts/tar-linux.sh caminho/do/feathercad.deb [pasta-de-saída]
set -e
deb=$(realpath "$1"); saida=$(realpath "${2:-.}")
versao=$(dpkg-deb -f "$deb" Version)
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
dpkg-deb -x "$deb" "$tmp/deb"
p="$tmp/feathercad-$versao"; mkdir -p "$p/icones"
cp "$tmp/deb/usr/bin/feathercad" "$p/"
for t in 32 64 128 512; do cp "$tmp/deb/usr/share/icons/hicolor/${t}x${t}/apps/feathercad.png" "$p/icones/feathercad-$t.png"; done
cp "$tmp/deb/usr/share/icons/hicolor/256x256@2/apps/feathercad.png" "$p/icones/feathercad-256.png"
cp "$tmp/deb/usr/share/applications/FeatherCAD.desktop" "$p/feathercad.desktop"
cp "$(dirname "$0")/../LICENSE" "$p/"
cat > "$p/instalar.sh" <<'EOF'
#!/bin/sh
# Instala o FeatherCAD só para este usuário (~/.local), sem administrador.
# Precisa da WebKitGTK 4.1 (libwebkit2gtk-4.1-0), que já vem na maioria das distribuições.
set -e
cd "$(dirname "$0")"
mkdir -p ~/.local/bin ~/.local/share/applications
install -m755 feathercad ~/.local/bin/feathercad
sed "s#^Exec=feathercad#Exec=$HOME/.local/bin/feathercad#" feathercad.desktop > ~/.local/share/applications/feathercad.desktop
for t in 32 64 128 256 512; do
  mkdir -p ~/.local/share/icons/hicolor/${t}x${t}/apps
  cp icones/feathercad-$t.png ~/.local/share/icons/hicolor/${t}x${t}/apps/feathercad.png
done
update-desktop-database ~/.local/share/applications 2>/dev/null || true
echo "FeatherCAD instalado. Procure por \"FeatherCAD\" no menu de aplicativos."
EOF
chmod 755 "$p/instalar.sh"
tar -czf "$saida/feathercad-linux-x86_64.tar.gz" -C "$tmp" "feathercad-$versao"
echo "$saida/feathercad-linux-x86_64.tar.gz"
