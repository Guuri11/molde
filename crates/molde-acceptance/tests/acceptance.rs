//! Ejecuta los .feature de la raíz. Un escenario @hitoN con pasos sin implementar cuenta como fallo:
//! así `cargo test` dice de verdad qué hitos están terminados.

mod lsp_client;
mod steps;
mod world;

use cucumber::World as _;

#[tokio::main(flavor = "multi_thread", worker_threads = 16)]
async fn main() {
    world::MoldeWorld::cucumber()
        .fail_on_skipped_with(|_, _, scenario| scenario.tags.iter().any(|t| t.starts_with("hito")))
        .run_and_exit(concat!(env!("CARGO_MANIFEST_DIR"), "/../../features"))
        .await;
}
