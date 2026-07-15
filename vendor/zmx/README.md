# ZMX

Grove embeds the official ZMX 0.6.0 release archives from
<https://github.com/neurosnap/zmx/releases/tag/v0.6.0>. ZMX is distributed
under the MIT license in `LICENSE`.

| Archive | SHA-256 |
| --- | --- |
| `zmx-0.6.0-linux-aarch64.tar.gz` | `c23f4b4ca80e144e329d042b91aae4859d23217ab07076b383af4134d97faac5` |
| `zmx-0.6.0-linux-x86_64.tar.gz` | `309d913b982ae16eac2a854f411de40eccc0b64afed892aa02a0be351f0271c1` |
| `zmx-0.6.0-macos-aarch64.tar.gz` | `3f070c6e38cb3a48ddc131dbe956fd4c4ebf4ca6cfcc57c3acbb40994f169787` |
| `zmx-0.6.0-macos-x86_64.tar.gz` | `1e6a3e5640b85332fac958aa4b1fc76390bc1f698b6b4975459d89f3bcfb1865` |

To update ZMX, replace all four archives and the license from one upstream
release, update the version and checksums here, then update `ZMX_VERSION` in
`src/session.rs`.
