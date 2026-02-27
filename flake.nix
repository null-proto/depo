{
  description = "Depo cload runner";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs = { self, nixpkgs }:
	let 
	system = "x86_64-linux";

	pkgs = import nixpkgs { inherit system; };
	in {
		devShells.${system} = {
			rust_up = pkgs.mkShellNoCC.override { stdenv = pkgs.llvmPackages.stdenv; } {
				buildInputs = with pkgs; [ rustup ];
			};

			default = pkgs.mkShellNoCC.override { stdenv = pkgs.llvmPackages.stdenv; } {
				buildInputs = with pkgs; [
					cargo
					rustc
					rustfmt
					rust-analyzer
				];
			};
		};
  };
}
