// Evita abrir uma janela de console extra no Windows em builds de release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    sparksai_lib::run()
}
