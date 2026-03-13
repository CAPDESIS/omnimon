# OmniMon v4 - Guía de Inicio Rápido

OmniMon es un monitor de sistema multiplataforma construido con Tauri, Rust y Svelte.

---

## 🚀 Inicio Rápido

### Para Usuarios (Ejecutar la Aplicación)

Si solo quieres **ejecutar OmniMon en modo desarrollo**:

1. **Instala prerequisitos** (solo la primera vez):
   ```powershell
   .\instalar-todo.ps1
   ```

2. **Ejecuta la aplicación**:
   ```powershell
   .\EJECUTAR_OMNIMON.bat
   ```

📖 **Guía detallada:** [EJECUTAR_DEV.md](./EJECUTAR_DEV.md)

---

### Para Desarrolladores (Contribuir al Proyecto)

Si quieres **desarrollar** o **contribuir** a OmniMon:

1. **Instala prerequisitos:**
   - Node.js (v18+)
   - Rust (cargo + rustc)
   - Visual Studio Build Tools (Windows)
   - Bun (opcional)

   📖 **Guía de instalación:** [INSTALACION_PREREQUISITOS.md](./INSTALACION_PREREQUISITOS.md)

2. **Clona el repositorio:**
   ```bash
   git clone <repo-url>
   cd omnimon/v4
   ```

3. **Instala dependencias:**
   ```bash
   cd apps/desktop
   npm install  # o bun install
   ```

4. **Ejecuta en modo desarrollo:**
   ```bash
   npm run tauri dev  # o bun run tauri dev
   ```

---

## 📚 Documentación

| Documento | Descripción |
|-----------|-------------|
| [EJECUTAR_DEV.md](./EJECUTAR_DEV.md) | Guía para ejecutar OmniMon en modo desarrollo |
| [INSTALACION_PREREQUISITOS.md](./INSTALACION_PREREQUISITOS.md) | Instalación de herramientas necesarias (Node, Rust, VS Tools) |
| [CLAUDE.md](../CLAUDE.md) | Instrucciones para Claude Code (contribución con IA) |

---

## 🛠️ Scripts Disponibles

| Script | Descripción |
|--------|-------------|
| `EJECUTAR_OMNIMON.bat` | Ejecuta OmniMon en modo desarrollo (doble click) |
| `instalar-todo.ps1` | Instala todos los prerequisitos automáticamente |
| `instalar-rust.ps1` | Instala solo Rust/Cargo |
| `instalar-bun.ps1` | Instala solo Bun (package manager) |
| `run-dev.ps1` | Script de desarrollo (requiere Developer PowerShell) |
| `run-dev-auto.ps1` | Script de desarrollo con auto-detección de herramientas |

---

## 🔍 Prerequisitos

### Esenciales (CRÍTICOS)
- ✅ **Node.js** v18+ - Runtime de JavaScript
- ✅ **Rust** (cargo + rustc) - Compilador para Tauri
- ✅ **Visual Studio Build Tools** - Herramientas de compilación C++ (solo Windows)

### Opcionales
- 📦 **Bun** - Package manager alternativo (más rápido que npm)

### ¿Cómo verificar si los tienes instalados?

```powershell
node --version
cargo --version
```

Si alguno falla, ejecuta `.\instalar-todo.ps1`

---

## 🎯 Estructura del Proyecto

```
v4/
├── apps/
│   └── desktop/           # Aplicación Tauri principal
│       ├── src/           # Frontend (Svelte)
│       └── src-tauri/     # Backend (Rust)
├── crates/
│   └── core/              # Lógica principal de monitoreo
├── EJECUTAR_OMNIMON.bat   # Script de ejecución fácil
├── EJECUTAR_DEV.md        # Guía de ejecución
├── INSTALACION_PREREQUISITOS.md  # Guía de instalación
├── instalar-todo.ps1      # Instalador automático
└── README.md              # Este archivo
```

---

## 🐛 Troubleshooting

### "cargo: command not found"
- **Solución:** Rust no está instalado o no está en el PATH
- **Fix:** Ejecuta `.\instalar-rust.ps1` o `.\instalar-todo.ps1`

### "tauri: command not found"
- **Solución:** Dependencias npm no instaladas
- **Fix:** Ejecuta `npm install` en `apps/desktop`

### "LINK: fatal error LNK1181" o "linker `link.exe` not found"
- **Solución:** Visual Studio Build Tools no instalado
- **Fix:** Instala desde https://visualstudio.microsoft.com/visual-cpp-build-tools/
- **Importante:** Selecciona "Desktop development with C++"

### La app no abre
- **Verifica prerequisitos:** `.\instalar-todo.ps1` mostrará qué falta
- **Ejecuta como Admin:** Algunas funciones (network capture) requieren permisos de administrador

---

## 🌟 Funcionalidades de OmniMon

- 📊 Monitoreo de CPU, RAM, Disco en tiempo real
- 🌐 Análisis de conexiones de red
- 🔍 Explorador de procesos con kill inteligente
- 🌐 Monitoreo de tabs de navegador (Chrome/Edge/Brave)
- 🎨 Temas claro/oscuro
- ⚡ Global hotkey (Ctrl+Alt+O)
- 📍 System tray con tooltip dinámico
- 🔔 Sistema de alertas
- 📈 Gráficas de métricas en tiempo real

---

## 📝 Notas de Desarrollo

- **Package Manager Preferido:** Bun (según CLAUDE.md)
- **Convención de Commits:** Conventional Commits (feat/fix/chore/etc.)
- **Testing:** `bun run test` y `bun run test:coverage`
- **Build:** `bun run tauri build -- --debug --no-bundle`

---

## 🤝 Contribuir

1. Lee [CLAUDE.md](../CLAUDE.md) para instrucciones específicas
2. Asegúrate de que los tests pasen: `npm run test`
3. Verifica que compila: `cargo check --workspace`
4. Sigue las convenciones de commits
5. **NUNCA** incluyas `Co-Authored-By: Claude` en los commits

---

## 📄 Licencia

[Ver licencia del proyecto principal]

---

## 🆘 Ayuda

Si tienes problemas:

1. Revisa [INSTALACION_PREREQUISITOS.md](./INSTALACION_PREREQUISITOS.md)
2. Consulta [EJECUTAR_DEV.md](./EJECUTAR_DEV.md)
3. Verifica que todos los prerequisitos estén instalados
4. Ejecuta `.\instalar-todo.ps1` para verificación automática

---

**¿Listo para empezar?** Ejecuta `.\EJECUTAR_OMNIMON.bat` 🚀
