g++ -std=c++17 -O3 VC-cs21btech11001.cpp -lzmq -o VC
g++ -std=c++17 -O3 SK-cs21btech11001.cpp -lzmq -o SK
for t in {10..15}; do
echo $t 5 1.5 50 > inp-params.txt
    declare -i m=$t-1
    for j in $(seq 1 $m) ; do
        declare -i q=$j+1
        echo $j $q >> inp-params.txt
    done
    echo $t 1 >> inp-params.txt

    ./VC >> vc.csv
    ./SK >> sk.csv

done

python3 plot.py
