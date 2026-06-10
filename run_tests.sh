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

echo "========================================="
echo "Running Lox interpreter tests"
echo "========================================="

# Проверяем наличие директории files
if [ ! -d "./files" ]; then
    echo -e "${RED}Error: ./files directory not found!${NC}"
    exit 1
fi

# Запускаем все .lox файлы
for file in ./files/*.lox; do
    if [ -f "$file" ]; then
        TOTAL=$((TOTAL + 1))
        filename=$(basename "$file")

        echo -n "Testing $filename ... "

        # Запускаем интерпретатор в release режиме
        output=$(cargo run -r -- "$file" 2>&1)

        # Проверяем статус выполнения
        if [ $? -eq 0 ]; then
            echo -e "${GREEN}PASSED${NC}"
            PASSED=$((PASSED + 1))
        else
            echo -e "${RED}FAILED${NC}"
            echo "----------------------------------------"
            echo "Error output from $filename:"
            echo "$output"
            echo "----------------------------------------"
            FAILED=$((FAILED + 1))
        fi
    fi
done

# Выводим итоги
echo "========================================="
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"
echo -e "${YELLOW}Total:  $TOTAL${NC}"
echo "========================================="

# Возвращаем код ошибки если есть падения
if [ $FAILED -gt 0 ]; then
    exit 1
fi

exit 0
