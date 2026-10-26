# Aura Studio — Centros Estéticos

App de escritorio (Tauri) para centros estéticos: agenda de citas, ventas,
inventario y catálogo. Funciona 100% en local — no requiere internet ni una
cuenta en la nube para operar día a día.

## Stack

- Tauri v2 (Rust) + Next.js (App Router, exportación estática) + TypeScript + Tailwind
- SQLite embebido (`rusqlite`), un archivo de base de datos por instalación
- react-hook-form + Zod
- shadcn/ui + Framer Motion

## Prerrequisitos (una sola vez por máquina)

1. Node.js 20+ y npm.
2. Rust (vía [rustup](https://rustup.rs)):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

3. Dependencias de sistema de Tauri en Linux (Debian/Ubuntu):

```bash
sudo apt update && sudo apt install -y \
  libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

En macOS o Windows sigue la [guía oficial de prerrequisitos de Tauri](https://v2.tauri.app/start/prerequisites/).

## Setup

```bash
npm install
```

### Desarrollo (ventana nativa + recarga en caliente)

```bash
npm run tauri dev
```

### Build de producción (instalador nativo)

```bash
npm run tauri build
```

## Flujo

1. Al abrir la app por primera vez en un equipo, se muestra la pantalla de
   **configuración inicial**: nombre del negocio + usuario dueño/a (usuario y
   contraseña). Se guarda en la base de datos local del equipo.
2. En los siguientes inicios, la app muestra el **login** directamente (sin
   landing).
3. Entra al panel (`/admin`). Productos y ventas están siempre activos.
4. En `/admin/settings` activa los módulos que necesites (Agenda, Empleados,
   Servicios, Paquetes, Reportes) y crea usuarios de staff. Todos los módulos
   son libres en esta edición local.
5. Con Agenda activa, crea empleados y citas en `/admin/appointments` (al
   cobrar, abre Nueva venta precargada).
6. Gestiona el catálogo habilitado y registra ventas en `/admin/sales`.

Los datos (incluidas las fotos del catálogo) se guardan únicamente en el
directorio de datos de la app en ese equipo — no hay sincronización entre
dispositivos todavía. Eso está planeado como un servicio opcional de pago a
futuro (sincronización, backups, integraciones).

## Publicar un release

El workflow `.github/workflows/release.yml` compila instaladores para
Linux, Windows y macOS (Intel + Apple Silicon) y los publica como un
GitHub Release en borrador cada vez que subes un tag `vX.Y.Z`. La app
instalada revisa ese release (vía el plugin `updater`) para ofrecer
actualizarse sola.

### Única vez: generar la llave de firma del updater

```bash
npx tauri signer generate -w ~/.tauri/aura-studio-updater.key
```

Pide una contraseña para proteger el archivo y al final imprime una
**llave pública**. Pégala en `src-tauri/tauri.conf.json` →
`plugins.updater.pubkey` (reemplaza el placeholder
`PASTE_YOUR_GENERATED_PUBLIC_KEY_HERE`) y haz commit de ese cambio.

Luego, en GitHub → **Settings → Secrets and variables → Actions → New
repository secret**, crea:

- `TAURI_SIGNING_PRIVATE_KEY`: contenido completo del archivo
  `~/.tauri/aura-studio-updater.key`.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: la contraseña que elegiste.

Guarda ese archivo `.key` en un lugar seguro (ej. un gestor de
contraseñas) — si lo pierdes no podrás firmar nuevas actualizaciones para
usuarios que ya instalaron una versión firmada con él.

### Cada release

1. Sube la versión en `src-tauri/tauri.conf.json` (`"version"`).
2. Crea y sube el tag correspondiente:
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```
3. En GitHub → **Actions**, espera a que terminen los 4 builds (~10-15 min).
4. En GitHub → **Releases**, revisa el borrador generado (trae los
   instaladores de cada sistema operativo + `latest.json` para el
   updater) y publícalo.
5. Enlaza los instaladores desde tu landing, o simplemente apunta a la
   página de Releases del repo.

### Notas

- macOS: sin una cuenta de Apple Developer (de pago) el `.dmg` queda sin
  firmar — al abrirlo la primera vez hay que hacer clic derecho → Abrir.
- Windows: el instalador NSIS se personaliza (imágenes de marca, licencia)
  en `src-tauri/tauri.conf.json` → `bundle.windows.nsis`.
- Para probar el auto-update de verdad: instala la v0.1.0, luego publica
  una v0.1.1 siguiendo los mismos pasos — la app instalada debería
  detectarla sola al abrir el panel, o al pulsar "Buscar actualizaciones"
  en Ajustes.

## Licencia

Este proyecto se distribuye bajo la licencia [MIT](LICENSE).
