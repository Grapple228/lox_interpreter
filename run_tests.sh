#!/bin/sh

# Цвета для вывода
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Парсим аргументы
SAVE_RESULT=false
for arg in "$@"; do
    if [ "$arg" = "--save" ]; then
        SAVE_RESULT=true
        break
    fi
done

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
    echo "${RED}Error: ./files directory not found!${NC}"
    exit 1
fi

# Создаем директорию для результатов если нужно
if [ "$SAVE_RESULT" = true ]; then
    mkdir -p ./test_results
    TIMESTAMP=$(date +"%Y-%m-%d_%H-%M-%S")
    RESULT_FILE="./bench_results/result-${TIMESTAMP}.log"
fi

# Находим максимальную длину имени файла
MAX_LEN=0
for file in ./files/*.lox; do
    if [ -f "$file" ]; then
        filename=$(basename "$file")
        len=$(echo "$filename" | wc -c)
        len=$((len - 1))
        if [ $len -gt $MAX_LEN ]; then
            MAX_LEN=$len
        fi
    fi
done

# Заголовок в файл результатов
if [ "$SAVE_RESULT" = true ]; then
    echo "=========================================" > "$RESULT_FILE"
    echo "Lox Interpreter Test Results" >> "$RESULT_FILE"
    echo "Date: $(date '+%Y-%m-%d %H:%M:%S')" >> "$RESULT_FILE"
    echo "=========================================" >> "$RESULT_FILE"
    echo "" >> "$RESULT_FILE"
fi

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
            if [ "$SAVE_RESULT" = true ]; then
                # echo "$filename: PASSED ($time_str)" >> "$RESULT_FILE"
                printf "%-${MAX_LEN}s ... %s (%s)\n" "$filename" "PASSED" "$time_str" >> "$RESULT_FILE"
            fi
        else
            printf "${RED}FAILED${NC} (%s)\n" "$time_str"
            echo "----------------------------------------"
            echo "Error output from $filename:"
            echo "$output"
            echo "----------------------------------------"
            FAILED=$((FAILED + 1))

            if [ "$SAVE_RESULT" = true ]; then
                echo "$filename: FAILED ($time_str)" >> "$RESULT_FILE"
                echo "----------------------------------------" >> "$RESULT_FILE"
                echo "Error output from $filename:" >> "$RESULT_FILE"
                echo "$output" >> "$RESULT_FILE"
                echo "----------------------------------------" >> "$RESULT_FILE"
            fi
        fi
    fi
done

AVG_TIME=$((TOTAL_TIME / TOTAL))

# Выводим итоги
echo "========================================="
echo "${GREEN}Passed: $PASSED${NC}"
echo "${RED}Failed: $FAILED${NC}"
echo "${YELLOW}Total:  $TOTAL${NC}"
echo "${YELLOW}Total time: ${TOTAL_TIME} ms${NC} (avg: ${AVG_TIME} ms)"
echo "========================================="

# Сохраняем итоги в файл
if [ "$SAVE_RESULT" = true ]; then
    echo "" >> "$RESULT_FILE"
    echo "=========================================" >> "$RESULT_FILE"
    echo "Passed: $PASSED" >> "$RESULT_FILE"
    echo "Failed: $FAILED" >> "$RESULT_FILE"
    echo "Total:  $TOTAL" >> "$RESULT_FILE"
    echo "Total time: ${TOTAL_TIME} ms (avg: ${AVG_TIME} ms)" >> "$RESULT_FILE"
    echo "=========================================" >> "$RESULT_FILE"

    echo ""
    echo "${YELLOW}Results saved to: ${RESULT_FILE}${NC}"
fi

# Возвращаем код ошибки если есть падения
if [ $FAILED -gt 0 ]; then
    exit 1
fi

exit 0
