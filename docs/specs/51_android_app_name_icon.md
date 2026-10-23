# Spec: Correção do Nome do App e Ícone no Android - Lauda (#51)

## Contexto
O APK Android do Lauda gerava metadados identificando o aplicativo como `lauda-client` / `LaudaClient` (`application-label` e `launchable-activity.label`) e a atividade principal (`MainActivity`) não tinha o ícone explicitamente associado (`icon=''`), fazendo com que alguns launchers do Android exibissem o ícone padrão do robô verde ou o nome técnico da crate Rust.

## Objetivo
Configurar os manifestos, strings de recursos e configurações do build Dioxus/Android para que o aplicativo seja nomeado canonicamente como **Lauda** e tenha o ícone oficial `@mipmap/ic_launcher` associado tanto à `<application>` quanto à `<activity>` principal (`launchable-activity`).

## Critérios de Aceitação
1. `Dioxus.toml` e `strings.xml` configurados com nome de exibição `Lauda`.
2. Script de build / injeção de manifesto configurando:
   - `android:label="Lauda"` na `<application>` e na `<activity>`
   - `android:icon="@mipmap/ic_launcher"` na `<application>` e na `<activity>`
   - `android:roundIcon="@mipmap/ic_launcher_round"` na `<application>`
3. Teste automatizado validando que o `AndroidManifest.xml` e/ou script de empacotamento declarem os atributos corretos de label `Lauda` e ícone na `launchable-activity`.
