# pve-rust

Rust SDK for the Proxmox Virtual Environment (PVE) API. Generated
from the upstream `apidoc.js` via [openapi-generator-cli][gen] with
custom Mustache template overrides.

> **Not an official Proxmox project.** Community SDK derived from the
> upstream `apidoc.js`. Always verify against
> <https://pve.proxmox.com/pve-docs/api-viewer/>.

Targets `reqwest` + `serde`. Requires Rust ≥ 1.75 (2021 edition).

## Install

Published to [crates.io](https://crates.io/crates/clientapi-pve) on
every GitHub release:

```bash
cargo add clientapi-pve
```

Or pin the version explicitly:

```toml
# Cargo.toml
[dependencies]
clientapi-pve = "*"
```

The import path is the snake_case form of the crate name, e.g.
`use clientapi_pve::apis::nodes_api;`.

## Usage

The Rust SDK uses module-level functions per tag — there's no Client
struct facade because Rust modules don't bind to method receivers.
Instead, hold a `Configuration` and pass it into each call:

```rust
use clientapi_pve::apis::{configuration::Configuration, qemu_api, nodes_api};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cfg = Configuration::new();
    cfg.base_path = "https://pve1.example.com:8006/api2/json".into();
    cfg.api_key = Some(clientapi_pve::apis::configuration::ApiKey {
        prefix: None,
        key: "PVEAPIToken=user@realm!tokenid=uuid-secret".into(),
    });

    let status = qemu_api::qemu_vm_status(&cfg, "pve1", 100).await?;
    let nodes  = nodes_api::nodes_get_nodes(&cfg).await?;
    println!("{:?}", status);
    Ok(())
}
```

## Compound configs

PVE encodes many fields as CLI-style shorthand strings
(`net0=virtio,bridge=vmbr0,firewall=1`). Round-trip helpers are
emitted for every compound config schema:

```rust
use clientapi_pve::models::PveQemuNetConfig;

let cfg = PveQemuNetConfig {
    model: "virtio".into(),
    bridge: Some("vmbr0".into()),
    firewall: Some("1".into()),
    ..Default::default()
};
let shorthand = cfg.to_shorthand();
// → "virtio,bridge=vmbr0,firewall=1"
```

## Indexed families

Numbered properties (`net0..net31`, `mp0..mp255`, …) are exposed on
every model as a single `Option<HashMap<u32, ItemType>>` field with
manual `Serialize`/`Deserialize` impls that round-trip the prefixed
wire keys:

```rust
use std::collections::HashMap;

let mut nets: HashMap<u32, models::PveQemuNetField> = HashMap::new();
nets.insert(0, models::PveQemuNetField::default());
nets.insert(3, models::PveQemuNetField::default());

let req = QemuCreateVmRequest {
    nets: Some(nets),
    ..Default::default()
};
// Wire format: { "net0": ..., "net3": ... }
```

## License

Apache 2.0 — see [LICENSE](./LICENSE).

[gen]: https://openapi-generator.tech
