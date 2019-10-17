{ pkgs, config }:
let
 name = "l3h-release-hook-publish";

 script = pkgs.writeShellScriptBin name ''
set -euox pipefail
echo "packaging for crates.io"
# order is important here due to dependencies
for crate in \
 crypto_api \
 detach \
 sodium \
 zombie_actor \
 ghost_actor \
 lib3h_protocol \
 mdns \
 p2p_protocol \
 lib3h
do
 cargo publish --manifest-path "crates/$crate/Cargo.toml"

 sleep 10
done
'';
in
{
 buildInputs = [ script ];
}
