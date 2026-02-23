//! Benchmark CPU très simple : calcul de nombres premiers.

pub fn cpu_test() -> u64 {
    // Compte le nombre de nombres premiers jusqu’à 100 000.
    let limit = 100_000usize;
    let mut count = 0usize;
    for n in 2..=limit {
        if is_prime(n) {
            count += 1;
        }
    }
    // Convertit le nombre de primes en un « score » (plus c’est grand, meilleur).
    count as u64
}

fn is_prime(n: usize) -> bool {
    if n <= 3 {
        return n > 1;
    }
    if n % 2 == 0 || n % 3 == 0 {
        return false;
    }
    let mut i = 5usize;
    while i * i <= n {
        if n % i == 0 || n % (i + 2) == 0 {
            return false;
        }
        i += 6;
    }
    true
}
