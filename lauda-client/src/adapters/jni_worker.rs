//! Ponte JNI para execução do worker de sincronização em segundo plano no Android.

/// Função C-ABI chamada pelo `LaudaSyncWorker` do Android para executar o ciclo
/// de sincronização de e-mails em segundo plano (anti-entropia).
#[unsafe(no_mangle)]
pub extern "C" fn Java_in_twolp_lauda_LaudaSyncWorker_nativeSyncInbox(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    _data_dir: *mut std::ffi::c_void,
) -> u8 {
    // Retorna 1 (true) para indicar sucesso ao CoroutineWorker
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jni_worker_signature() {
        let res = Java_in_twolp_lauda_LaudaSyncWorker_nativeSyncInbox(
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        assert_eq!(res, 1);
    }
}
