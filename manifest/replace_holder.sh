cat $1 | while read -r line; do echo "$line" | envsubst; done