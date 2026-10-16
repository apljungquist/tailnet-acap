# Tailnet for ACAP

> [!NOTE]
> This app is built on [`tailscale-rs`](https://github.com/tailscale/tailscale-rs), which is work on progress;
> Expect bugs and missing features from the library, and for this application to add a few more.

This app prioritizes having a small footprint; when installed it uses 3.3MiB and 2.3MiB on aarch64 and armv7hf of storage respectively [^1] [^2].

[^1]: The file system on Axis devices is compressed so this is measured by comparing `df` before and after installing the app.
[^2]: Measured on commit d36ab2bc26743432bccaf979dd8437785c24f583
