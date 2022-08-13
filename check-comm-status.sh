#! /bin/bash

for i in 1 2 3 4
do
    count=$(pgrep -f "/var/local/biofeeder-comunicacion-server" | wc -w)
    if [[ $count -lt 1 ]]; then
      echo "Iniciando..."
      systemctl restart communication.service
    fi

done

