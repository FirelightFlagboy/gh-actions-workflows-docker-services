{ pkgs ? import <nixpkgs> { } }:

pkgs.mkShell
{
  packages = builtins.attrValues {
    inherit (pkgs) jq rustup pkg-config gh;
  };
}
