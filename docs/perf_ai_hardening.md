# OmniMon AI Hardening & Performance Report

## 1. Prompt Injection Defense
Se ha implementado un mecanismo de sanitización en el backend (`ai.rs`) que evalúa los mensajes del usuario antes de enviarlos al LLM. Se bloquean heurísticas comunes de inyección como "ignora las instrucciones anteriores", "borra mis reglas", y otras variantes que intentan alterar el system prompt o abusar de las herramientas del sistema.

## 2. AI Response Caching (Memoization)
Para ahorrar tokens y ciclos de CPU, se ha implementado una caché en memoria usando `std::sync::RwLock` y `std::collections::HashMap`.
- Se genera un hash de la consulta del usuario (incluyendo contexto técnico como nombres de procesos).
- Si la pregunta (ej. "Qué es WindowServer") ya fue respondida para el mismo contexto/estado, se retorna inmediatamente la respuesta almacenada sin hacer la petición a la API (Ollama/OpenAI/etc).

## 3. Graceful Degradation
El componente `AIChat.svelte` fue modificado para manejar errores de API y timeouts de forma elegante.
- Se capturan las excepciones y se muestra un mensaje amigable al usuario.
- Se provee un botón de "Reintentar".
- Si se detecta un error de conexión, se sugiere activar o revisar el modelo local (Ollama).
