echo -n > elap.txt
for k in 5 10 15 20 25 ; do
    echo 4 $k 5 5 > inp-params.txt
    python3 scr.py $1
    sleep 10
    declare u=`python3 plot2.py $1`
    echo $u
    echo $u >> elap.txt
done