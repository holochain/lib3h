{ pkgs }:
let
  name = "l3h-test";

  script = pkgs.writeShellScriptBin name
  ''
  echo BACKTRACE_STRATEGY=$BACKTRACE_STRATEGY
  hn-rust-fmt-check \
  && hn-rust-clippy \
  && RUST_BACKTRACE=1 RUST_LOG=trace cargo test test_mirror_suite -- --nocapture
  '';
in
{
 buildInputs = [ script ];
}
