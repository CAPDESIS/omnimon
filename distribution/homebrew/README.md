# OmniMon Homebrew Tap (macOS)

Esta guía explica cómo distribuir OmniMon para usuarios de macOS (Apple Silicon y x86_64) utilizando **Homebrew Casks**.

## Paso 1: Crear el repositorio Tap

Para que los usuarios puedan instalar OmniMon usando `brew tap chochy2001/omnimon`, necesitas crear un repositorio dedicado en GitHub.

1. Ve a GitHub y crea un nuevo repositorio público.
2. El repositorio **DEBE** llamarse `homebrew-omnimon` (el prefijo `homebrew-` es obligatorio en el ecosistema Brew).
3. Clona el repositorio recién creado en tu máquina local.

## Paso 2: Copiar y configurar la fórmula

1. Toma el archivo `omnimon.rb` que se encuentra en esta carpeta (`distribution/homebrew/omnimon.rb`).
2. Cópialo al directorio raíz (o dentro de una carpeta `Casks/`) de tu nuevo repositorio `homebrew-omnimon`.
3. **Punto Crítico:** Cada vez que liberes una nueva versión:
   - Actualiza el campo `version` (ej. `"4.0.4"`).
   - Actualiza el campo `sha256` calculando el checksum del archivo `.dmg` subido al Release:
     ```bash
     shasum -a 256 OmniMon_4.0.4_x64.dmg
     ```
   - Actualiza la `url` si el nombre de tu binario en GitHub Releases cambia dependiendo de la arquitectura (Tauri genera esto por ti).

## Paso 3: Publicar el Tap

1. Haz un commit de tu archivo `.rb` en el repositorio `homebrew-omnimon` y empújalo a la rama `main` o `master`.
2. ¡Listo! 

## Paso 4: Instrucciones para el usuario

Cualquier usuario de macOS ahora podrá instalar OmniMon ejecutando:

```bash
brew tap chochy2001/omnimon
brew install --cask omnimon
```

### Actualizaciones Automáticas

Homebrew permite actualizar los Casks. Cuando el usuario ejecute `brew upgrade`, Homebrew descargará la última versión definida en tu tap. Asegúrate de actualizar la fórmula `.rb` en el repositorio `homebrew-omnimon` con cada release de CI/CD.