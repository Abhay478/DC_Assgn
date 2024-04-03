echo -n > mc.txt
for n in 4 9 16 25 ; do
    echo $n 15 5 5 > inp-params.txt
    python3 scr.py $1
    sleep $((2*$n+$n*$n-$n*$n*$1 ))
    declare u=`python3 plot1.py $1`
    echo $u
    # read -r eh
    echo $u >> mc.txt
done