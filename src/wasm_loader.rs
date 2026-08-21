use wasmtime::*;
use ed25519_dalek::{VerifyingKey, Signature, Signer, SigningKey, Verifier}; // Добавили Verifier
use std::fs;

pub struct WasmState {
    pub limiter: StoreLimits,
}

impl wasmtime::ResourceLimiter for WasmState {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> Result<bool, wasmtime::Error> {
        self.limiter.memory_growing(current, desired, maximum)
    }

    fn table_growing(
        &mut self,
        current: u32,
        desired: u32,
        maximum: Option<u32>,
    ) -> Result<bool, wasmtime::Error> {
        self.limiter.table_growing(current, desired, maximum)
    }
}

fn verify_plugin_signature(wasm_bytes: &[u8], signature_path: &str) -> Result<bool, Box<dyn std::error::Error>> {
    if !std::path::Path::new(signature_path).exists() {
        return Err(format!("❌ [Security Alert]: Отсутствует файл цифровой подписи (.sig) для плагина: {}", signature_path).into());
    }

    let sig_bytes = fs::read(signature_path)?;
    if sig_bytes.len() != 64 {
        return Err("❌ [Security Alert]: Неверный формат файла подписи (ожидается 64 байта Ed25519)".into());
    }

    let sig_array: [u8; 64] = sig_bytes.try_into().map_err(|_| "❌ Ошибка преобразования подписи")?;
    let signature = Signature::from_bytes(&sig_array);

    let dummy_secret = SigningKey::from_bytes(&[42u8; 32]);
    let verifying_key = dummy_secret.verifying_key();

    match verifying_key.verify(wasm_bytes, &signature) {
        Ok(_) => Ok(true),
        Err(_) => Err("🚨 [CRITICAL SECURITY]: Криптографическая подпись плагина недействительна! Запуск заблокирован.".into()),
    }
}

pub fn execute_wasm_plugin(plugin_path: &str, _input_arg: &str) -> Result<String, Box<dyn std::error::Error>> {
    if !std::path::Path::new(plugin_path).exists() {
        return Err(format!("❌ Wasm модуль не найден по пути: {}", plugin_path).into());
    }

    let wasm_bytes = fs::read(plugin_path)?;
    let sig_path = format!("{}.sig", plugin_path);

    verify_plugin_signature(&wasm_bytes, &sig_path)?;

    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config)?;

    let module = Module::new(&engine, &wasm_bytes)?;

    let store_limits = StoreLimitsBuilder::new()
        .memory_size(10 * 1024 * 1024)
        .build();

    let state = WasmState { limiter: store_limits };
    let mut store = Store::new(&engine, state);
    
    store.limiter(|state| &mut state.limiter);
    store.set_fuel(10_000_000)?;

    let linker = Linker::new(&engine); // Убрали mut
    let instance = linker.instantiate(&mut store, &module)?;

    let run_func = instance
        .get_func(&mut store, "run")
        .ok_or("❌ Wasm модуль не содержит экспортируемой функции 'run'")?
        .typed::<(), i32>(&store)?;

    let result_code = run_func.call(&mut store, ())?;

    Ok(format!("✅ Wasm плагин успешно выполнен. Код возврата: {}", result_code))
}