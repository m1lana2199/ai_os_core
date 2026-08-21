# Используем официальный образ Rust для сборки
FROM rust:1.75 as builder

WORKDIR /app
COPY . .

# Сборка проекта в релизном режиме
RUN cargo build --release

# Финальный легковесный образ для продакшена
FROM debian:bookworm-slim

WORKDIR /app

# Устанавливаем необходимые системные зависимости для работы утилит
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Копируем скомпилированный бинарник из билдера
COPY --from=builder /app/target/release/ai_os_core /app/ai_os_core

# Открываем порт для веб-клиента и API
EXPOSE 8080

# Запускаем наше приложение
CMD ["./ai_os_core"]