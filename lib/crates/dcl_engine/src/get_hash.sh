#!/bin/bash

echo "🔍 Finding and sorting .rs and .toml files..."
LC_ALL=C FILES=$(find . -type f \( -name '*.rs' -o -name '*.toml' \) | sort)

echo "📄 Sorted files:"
echo "$FILES"

echo
echo "🔑 Calculating SHA-256 of each file..."
while IFS= read -r file; do
  HASH=$(sha256sum "$file" | awk '{print $1}')
  echo "$HASH  ${file#./}"
done <<< "$FILES"

echo
echo "🧮 Final folder hash:"
find . -type f \( -name '*.rs' -o -name '*.toml' \) \
  | sort \
  | xargs sha256sum \
  | sed 's|  ./|  |' \
  | sha256sum \
  | awk '{print "🔒 Hash:", $1}'


# Step 3: Original hash logic
LC_ALL=C HASH=$(find . \
    -type f \( -name '*.rs' -o -name '*.toml' \) \
    -print0 \
    | sort -z \
    | xargs -0 sha256sum \
    | sha256sum \
    | awk '{print $1}')
echo "• Folder-hash = $HASH"
