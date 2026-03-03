# Integracion de IA

macmon v2 mantiene la ejecucion nativa y segura.

## Flujo de Datos

1. El usuario abre Preferencias y selecciona proveedor y modelo.
2. La API key se guarda con `SecItemAdd` en Keychain de macOS.
3. El usuario pulsa Smart Optimize en Process Picker.
4. Swift construye un snapshot ligero de procesos.
5. El snapshot se envia al proveedor de IA solo para analisis.
6. El proveedor devuelve JSON estricto con PIDs candidatos.
7. La UI marca filas sugeridas y pide aplicar o revisar.
8. Solo con aprobacion del usuario, los PIDs pasan a Bash.
9. Bash valida seguridad y envia `kill -15` con fallback `kill -9`.

## Reglas de Seguridad

1. La salida de IA se trata como entrada no confiable.
2. Solo se aceptan PIDs de la respuesta.
3. Ningun comando de IA se ejecuta.
4. Blocklist y proteccion de procesos Apple siempre se aplican.
5. Servicios criticos de audio y video permanecen protegidos.
6. Texto alucinado del LLM se limpia con extraccion de PIDs por regex.
7. Los PIDs sugeridos se validan contra procesos vivos y blocklist antes de mostrar confirmacion.

## Proveedores

1. OpenAI
2. Anthropic
3. OpenRouter

## Almacenamiento en Keychain

1. Service key: `com.macmon.ai`
2. Account key: nombre del proveedor (`openai`, `anthropic`, `openrouter`)
3. Valor: bytes de API key, sin guardado en texto plano
4. Accesibilidad: `kSecAttrAccessibleWhenUnlocked`
