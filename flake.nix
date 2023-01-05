{
  description = "XenceLabs Quick Keys unnofficial driver for linux";

  outputs = { self, nixpkgs }: let
    pkgs = nixpkgs.legacyPackages.x86_64-linux; 
  in {

    devShells.x86_64-linux.quick-keys = pkgs.callPackage ./shell.nix {};

    devShells.x86_64-linux.default = self.devShells.x86_64-linux.quick-keys;

  };
}
