let
  mozillaOverlay =
    import (builtins.fetchGit {
      url = "https://github.com/mozilla/nixpkgs-mozilla.git";
      rev = "57c8084c7ef41366993909c20491e359bbb90f54";
    });
  nixpkgs = import <nixpkgs> { overlays = [ mozillaOverlay ]; };
  rust-nightly = with nixpkgs; ((rustChannelOf { date = "2020-10-23"; channel = "nightly"; }).rust.override {
    targets = [ "wasm32-unknown-unknown" ];
  });
  checkScript = nixpkgs.writeShellScriptBin "check-env" ''
    echo "export from shell"
    echo LIBCLANG_PATH=$LIBCLANG_PATH
    echo LLVM_LIBRARY_DIR=$LLVM_LIBRARY_DIR
    echo " "
    echo "Find libclang.so in store"
    find /nix/store | grep libclang.so
    echo "Find LLVMgold.so in store"
    find /nix/store | grep LLVMgold.so
  '';
  clangStdenv = nixpkgs.llvmPackages_10.stdenv;
in
clangStdenv.mkDerivation {
  name = "clang-10-nix-shell";
  buildInputs = with nixpkgs; [
    cmake
    pkg-config
    rust-nightly

    # rust-ssvm
    llvmPackages_10.llvm
    llvmPackages_10.libclang
    lld_10
    boost
    checkScript
    protobuf
  ] ++ lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.Security
  ];
  LIBCLANG_PATH = "${nixpkgs.llvmPackages_10.libclang.lib}/lib";
  LLVM_LIBRARY_DIR = "${nixpkgs.llvmPackages_10.llvm.lib}/lib";
  PROTOC = "${nixpkgs.protobuf}/bin/protoc";
}
