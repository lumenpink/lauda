# Spec: Correção de Ícones Adaptativos e Expurgamento do Template Dioxus v24 (#76)

## Contexto
O aplicativo Lauda empacotado para Android (`lauda.apk`) não exibia o ícone canônico nos lançadores modernos de Android (API 26+).
A investigação estrutural identificou que o gerador de projetos Dioxus CLI (`dx build --platform android`) injeta o template padrão contendo `res/drawable-v24/ic_launcher_foreground.xml` (o robô verde padrão do SDK Android).
Como a resolução de recursos do Android dá prioridade estrita a qualificadores de versão (`drawable-v24/`) sobre o diretório padrão (`drawable/`), qualquer dispositivo Android 7.0+ (API 24+) ignora o arquivo `ic_launcher_foreground.png` e renderiza o vetor padrão do template.
Adicionalmente, os scripts de compilação expurgavam apenas arquivos WebP (`mipmap-*/*.webp`), deixando intacto o diretório `drawable-v24/`.

## Objetivo
1. Expurgar deterministamente `drawable-v24/ic_launcher_foreground.xml`, qualquer arquivo de recurso residual com prefixo `$ic_launcher_foreground` e o diretório `drawable-v24` na esteira de montagem do APK em `scripts/publish-nightly.sh`.
2. Expandir `scripts/gerar-recursos-android.py` para gerar o foreground adaptativo nas densidades padrão Android (`drawable-mdpi`, `drawable-hdpi`, `drawable-xhdpi`, `drawable-xxhdpi`, `drawable-xxxhdpi`) além de `drawable/ic_launcher_foreground.png`, garantindo nitidez sem distorção em todas as telas.
3. Atualizar a suíte de testes `tests/test-android-icons.sh` para auditar a ausência de qualificadores de template legados e a integridade de todas as densidades dos ícones adaptativos.

## Critérios de Aceitação
- `scripts/publish-nightly.sh` remove qualquer sobreposição de `drawable-v24` e arquivos vetoriais residuais do template Dioxus antes da montagem pelo Gradle.
- `scripts/gerar-recursos-android.py` gera e sincroniza todos os ativos adaptativos necessários em `lauda-client/android/res/`.
- `tests/test-android-icons.sh` valida com sucesso a presença e dimensões dos drawables adaptativos nas densidades correspondentes e rejeita qualquer resíduo de template.
- Testes locais e `cccp check` passam com 100% de conformidade.
