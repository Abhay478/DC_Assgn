mkdir -p out
g++ -std=c++17 -O3 -g src/VC-cs21btech11001.cpp -lzmq -o out/VC
g++ -std=c++17 -O3 -g src/SK-cs21btech11001.cpp -lzmq -o out/SK
for t in {10..15}; do
echo $t 5 1.5 50 > inp-params.txt
    # declare -i m=$t-1
    echo 1 2 $t >> inp-params.txt
    for j in $(seq 2 $t) ; do
        q=$(( $j % $t + 1 ))
        p=$(( $j-2 % $t + 1 ))
        echo $j $q $p >> inp-params.txt
    done

    out/VC >> vc.csv
    out/SK >> sk.csv

done

python3 plot.py
rm vc.csv sk.csv