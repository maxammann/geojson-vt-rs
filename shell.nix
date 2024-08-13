# This nix-shell supports macOS and Linux.
# The repository supports direnv (https://direnv.net/). If your IDE supports direnv,
# then you do not need to care about dependencies.

{ pkgs ? import <nixpkgs> {
    overlays = [];
  }
}:
with pkgs;
let
  unstable = import
    (builtins.fetchTarball {
      url = "https://github.com/NixOS/nixpkgs/archive/cb9a96f23c491c081b38eab96d22fa958043c9fa.tar.gz"; # Ger from here: https://github.com/NixOS/nixpkgs/tree/nixos-unstable
    })
    { };
in
(pkgs.mkShell.override {
  stdenv = llvmPackages_16.stdenv;
}) {
  nativeBuildInputs = [
    # Tools
    rustup
    pkgs.cargo-criterion
    unstable.nixpkgs-fmt # To format this file: nixpkgs-fmt *.nix
  ] ++ lib.optionals stdenv.isLinux [
  ];
}
