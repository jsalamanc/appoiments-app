# 12 — Portal público de reserva

**Estado:** MVP hecho  
**Dependencia:** módulo `public_booking` (+ Agenda y Servicios); migraciones `009` y `010`

## Qué es

Página pública `/r/[slug]` donde el cliente elige servicio, profesional, horario y deja sus datos para crear una cita.

## Implementado

1. RPCs `get_public_booking_catalog`, `get_public_busy_ranges`, `create_public_booking`
2. UI de reserva en pasos
3. Enlace copiable en Ajustes (dueño)

## Fuera de alcance (futuro)

- Pagos online
- Recordatorios email/WhatsApp
- Captcha / rate limit avanzado
