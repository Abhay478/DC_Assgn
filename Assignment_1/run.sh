g++ -std=c++17 -O3 -g VC-cs21btech11001.cpp -lzmq -o VC.out
g++ -std=c++17 -O3 -g SK-cs21btech11001.cpp -lzmq -o SK.out
for t in {10..15}; do
echo $t 5 1.5 50 > inp-params.txt
    declare -i m=$t-1
    for j in $(seq 1 $m) ; do
        declare -i q=$j+1
        echo $j $q >> inp-params.txt
    done
    echo $t 1 >> inp-params.txt

    ./VC.out >> vc.csv
    ./SK.out >> sk.csv

done

python3 plot.py
