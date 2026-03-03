# Integracion de IA

macmon soporta analisis opcional de procesos asistido por IA a traves de proveedores externos. La capa de IA es estrictamente de lectura — sugiere procesos para cerrar, pero nunca ejecuta comandos directamente.

## Como Funciona

1. Abre Preferencias desde la barra de menu y configura tu proveedor, modelo y API key
2. Las API keys se guardan de forma segura en el Keychain de macOS (nunca en archivos de texto)
3. Presiona "Smart Optimize" en el Process Picker
4. macmon envia un snapshot ligero de procesos al proveedor elegido
5. El proveedor devuelve una lista de PIDs candidatos como JSON
6. Los procesos sugeridos se resaltan en la tabla para tu revision
7. Tu eliges cuales cerrar — nada pasa sin aprobacion explicita
8. Los PIDs seleccionados pasan por las mismas validaciones de seguridad que la seleccion manual

## Seguridad

- La salida de IA se trata como entrada no confiable en cada paso
- Solo se extraen PIDs numericos de las respuestas — nunca se ejecutan comandos
- Los procesos protegidos (WindowServer, kernel_task, launchd, etc.) no pueden seleccionarse sin importar lo que sugiera la IA
- La verificacion de firma de codigo Apple aplica a todos los nombres de procesos del sistema

## Proveedores Soportados

| Proveedor | Endpoint |
|-----------|----------|
| OpenAI | `api.openai.com/v1/chat/completions` |
| Anthropic | `api.anthropic.com/v1/messages` |
| OpenRouter | `openrouter.ai/api/v1/chat/completions` |

## Almacenamiento en Keychain

- Service: `com.macmon.ai`
- Account: nombre del proveedor (`openai`, `anthropic`, `openrouter`)
- Accesibilidad: `kSecAttrAccessibleWhenUnlocked`
