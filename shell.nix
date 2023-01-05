{ pkgs, ... }:
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    pkg-config
    udev
  ];
  buildInputs = with pkgs; [
    cargo
    rustc
  ];
}
