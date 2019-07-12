{ pkgs, config }:
let
 name = "l3h-release-hook-version";

 script = pkgs.writeShellScriptBin name ''
for dep in \
 lib3h_protocol \
 lib3h_crypto_api \
 lib3h_sodium
do
 echo "bumping $dep dependency versions to ${config.release.version.current} in all Cargo.toml"
 find . \
  -name "Cargo.toml" \
  -not -path "**/target/**" \
  -not -path "**/.git/**" \
  -not -path "**/.cargo/**" | xargs -I {} \
  sed -i 's/^'"''${dep}"' = { version = "=[0-9]\+.[0-9]\+.[0-9]\+\(-alpha[0-9]\+\)\?"/'"''${dep}"' = { version = "=${config.release.version.current}"/g' {}
done
'';
in
{
 buildInputs = [ script ];
}
