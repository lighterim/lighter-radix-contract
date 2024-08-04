scrypto build
resim reset
result=$(resim new-account)
export admin_account=$(echo $result|grep "Account component address: "|awk -F ": " '{print $2}'|awk -F " " '{print $1}')
export admin_account_priv=$(echo $result|grep "Private key:" |awk -F "Private key: " '{print $2}'|awk -F " " '{print $1}')
export admin_account_badge=$(echo $result|grep "Owner badge: "|awk -F ": " '{print $2}'|awk -F " " '{print $1}')
export account=$admin_account
result=$(resim new-account)
export p1=$(echo $result|grep "Account component address: "|awk -F ": " '{print $2}'|awk -F " " '{print $1}')
export p1_priv=$(echo $result|grep "Private key:" |awk -F "Private key: " '{print $2}'|awk -F " " '{print $1}')
export p1_badge=$(echo $result|grep "Owner badge: "|awk -F ": " '{print $2}'|awk -F " " '{print $1}')
result=$(resim new-account)
export p2=$(echo $result|grep "Account component address: "|awk -F ": " '{print $2}'|awk -F " " '{print $1}')
export p2_priv=$(echo $result|grep "Private key:" |awk -F "Private key: " '{print $2}'|awk -F " " '{print $1}')
export p2_badge=$(echo $result|grep "Owner badge: "|awk -F ": " '{print $2}'|awk -F " " '{print $1}')
result=$(resim new-account)
export p3=$(echo $result|grep "Account component address: "|awk -F ": " '{print $2}'|awk -F " " '{print $1}')
export p3_priv=$(echo $result|grep "Private key:" |awk -F "Private key: " '{print $2}'|awk -F " " '{print $1}')
export p3_badge=$(echo $result|grep "Owner badge: "|awk -F ": " '{print $2}'|awk -F " " '{print $1}')


result=$(resim new-token-fixed --symbol=USDT --name=USDT 1000000)
# export usdt=$(echo $result | grep "Resource:" | awk -F " " '{print $3}')
export usdt=$(echo $result | awk -F "Resource: " '{print $2}')
result=$(resim new-token-fixed --symbol=USDC --name=USDC 1000000)
# export usdc=$(echo $result | grep "Resource:" | awk -F " " '{print $3}')
export usdc=$(echo $result | awk -F "Resource: " '{print $2}')
resim transfer $usdt:100000 $p2
resim transfer $usdc:100000 $p2
resim transfer $usdc:100000 $p3
resim transfer $usdt:200 $p1
resim transfer $usdc:200 $p1


result=$(resim publish ".")
export pkg=$(echo $result | awk -F ": " '{print $2}')


export ticket_price=10
export payment_window_epochs=8
export relay_pub_key="a5bc3d9296bda1e52f96bf0a65238998877dbddb0703bd37ef1f18a6ffce458a"
export domain_name="@lighter.im"
result=$(resim run < ./manifest/replace_holder.sh ./manifest/instantiate.rtm)
export component=$(echo $result | grep "Component: "| awk -F "Component: " '{print $2}' | awk -F " " '{print $1}')
export admin_badge=$(echo $result | grep "Resource: " | awk -F "Resource: " '{if (NR==1) print $2}' | awk -F " " '{print $1}')
export op_badge=$(echo $result | grep "Resource: " | awk -F "Resource: " '{if (NR==2) print $2}' | awk -F " " '{print $1}')
export ticket_addr=$(echo $result | grep "Resource: " | awk -F "Resource: " '{if (NR==3) print $2}' | awk -F " " '{print $1}')
export escrow_addr=$(echo $result | grep "Resource: " | awk -F "Resource: " '{if (NR==4) print $2}' | awk -F " " '{print $1}')

resim set-default-account $p1 $p1_priv $p1_badge
export amount=10
export account=$p1
export dns_name=dust@lighter.im
export nostr_pub_key=npub180czphasst2k8035u9nuk4wdmt9eppme9l03tp5j43qck7s8emwqr8agc0
export xrd="resource_sim1tknxxxxxxxxxradxrdxxxxxxxxx009923554798xxxxxxxxxakj8n3"
result=$(resim run < ./manifest/replace_holder.sh ./manifest/take_ticket.rtm)
export p1_ticket=$(echo $result | grep "ResAddr: " | awk -F "ResAddr: " '{if (NR==3) print $2}' | awk -F " " '{print $1}')
export p1_ticket_id="<dust_lighter_im>"

resim set-default-account $p2 $p2_priv $p2_badge
export amount=10
export account=$p2
export dns_name=dust2@lighter.im
export nostr_pub_key=npub180czphasst2k8035u9nuk4wdmt9eppme9l03tp5j43qck7s8emwqr8agc1
export xrd="resource_sim1tknxxxxxxxxxradxrdxxxxxxxxx009923554798xxxxxxxxxakj8n3"
result=$(resim run < ./manifest/replace_holder.sh ./manifest/take_ticket.rtm)
export p2_ticket=$(echo $result | grep "ResAddr: " | awk -F "ResAddr: " '{if (NR==3) print $2}' | awk -F " " '{print $1}')
export p2_ticket_id="<dust2_lighter_im>"

export seller=$p2
export seller_id=$p2_ticket_id
export seller_fee=40
export buyer=$p1
export buyer_id=$p1_ticket_id
export buyer_fee=20
export amount=1004
export volume=1000
export trade_id=1
export price=0.03
export payment_method=alipay
export res_addr=$xrd
export signature=$(python3 sig_util.py $trade_id $buyer_id $seller_id $res_addr $volume $price $buyer_fee $seller_fee $payment_method)
echo "$trade_id,$buyer_id,$seller_id,$res_addr,$volume,$price,$buyer_fee,$seller_fee,$payment_method"
result=$(resim run <./manifest/replace_holder.sh ./manifest/create_escrow.rtm)

resim set-default-account $p1 $p1_priv $p1_badge
result=$(resim run <./manifest/replace_holder.sh ./manifest/buyer_paid.rtm)

resim set-default-account $p2 $p2_priv $p2_badge
result=$(resim run <./manifest/replace_holder.sh ./manifest/seller_release.rtm)

resim set-default-account $p1 $p1_priv $p1_badge
export amount=998
result=$(resim run <./manifest/replace_holder.sh ./manifest/buyer_withdraw.rtm)


resim set-default-account $p1 $p1_priv $p1_badge
result=$(resim run <./manifest/replace_holder.sh ./manifest/buyer_cancel.rtm)
resim set-default-account $p2 $p2_priv $p2_badge
result=$(resim run <./manifest/replace_holder.sh ./manifest/seller_cancel.rtm)

resim set-current-epoch 10
result=$(resim run <./manifest/replace_holder.sh ./manifest/request_cancel.rtm)
resim set-current-epoch 18
result=$(resim run <./manifest/replace_holder.sh ./manifest/seller_cancel.rtm)
