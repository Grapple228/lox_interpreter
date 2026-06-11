#!/bin/bash

# Цвета для вывода
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Счетчики
TOTAL=0
PASSED=0
FAILED=0
TOTAL_TIME=0

cargo build --release --quiet

echo "========================================="
echo "Running Lox interpreter tests"
echo "========================================="

# Проверяем наличие директории files
if [ ! -d "./files" ]; then
    echo -e "${RED}Error: ./files directory not found!${NC}"
    exit 1
fi

# Находим максимальную длину имени файла
MAX_LEN=0
for file in ./files/*.lox; do
    if [ -f "$file" ]; then
        filename=$(basename "$file")
        len=${#filename}
        if [ $len -gt $MAX_LEN ]; then
            MAX_LEN=$len
        fi
    fi
done

# Запускаем все .lox файлы
for file in ./files/*.lox; do
    if [ -f "$file" ]; then
        TOTAL=$((TOTAL + 1))
        filename=$(basename "$file")

        # Выравнивание
        printf "Testing %-${MAX_LEN}s ... " "$filename"

        # Запускаем интерпретатор с замером времени
        start_time=$(date +%s%N)
        output=$(target/release/lox "$file" 2>&1)
        exit_code=$?
        end_time=$(date +%s%N)

        duration_ns=$((end_time - start_time))
        duration_ms=$((duration_ns / 1000000))
        duration_us=$((duration_ns / 1000 - duration_ms * 1000))

        TOTAL_TIME=$((TOTAL_TIME + duration_ms))

        # Всегда показываем ms и μs
        time_str="${duration_ms}.${duration_us} ms"

        # Проверяем статус выполнения
        if [ $exit_code -eq 0 ]; then
            printf "${GREEN}PASSED${NC} (%s)\n" "$time_str"
            PASSED=$((PASSED + 1))
        else
            printf "${RED}FAILED${NC} (%s)\n" "$time_str"
            echo "----------------------------------------"
            echo "Error output from $filename:"
            echo "$output"
            echo "----------------------------------------"
            FAILED=$((FAILED + 1))
        fi
    fi
done

AVG_TIME=$((TOTAL_TIME / TOTAL))

# Выводим итоги
echo "========================================="
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"
echo -e "${YELLOW}Total:  $TOTAL${NC}"
echo -e "${YELLOW}Total time: ${TOTAL_TIME} ms${NC} (avg: ${AVG_TIME} ms)"
echo "========================================="

# Возвращаем код ошибки если есть падения
if [ $FAILED -gt 0 ]; then
    exit 1
fi

exit 0
