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
