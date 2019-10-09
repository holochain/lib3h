{ pkgs }:
let
  name = "l3h-test";

  script = pkgs.writeShellScriptBin name
  ''
  echo BACKTRACE_STRATEGY=$BACKTRACE_STRATEGY
  RUST_LOG=trace RUST_BACKTRACE=1 \
  hn-rust-fmt-check \
  && hn-rust-clippy \
  && cargo test
  '';
in
{
 buildInputs = [ script ];
}
