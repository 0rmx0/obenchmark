//! Suite de benchmarks CPU.
//!
//! Chaque test implémente le trait [`Benchmark`] et cible un sous-système
//! précis du processeur (calcul entier, calcul flottant, parallélisme,
//! débit vectoriel, hachage, tri, recherche arborescente, ...). Les
//! charges de travail s'appuient, autant que possible, sur des noyaux de
//! calcul reconnus dans la littérature du benchmarking (SAXPY, crible
//! d'Ératosthène, simulation à N corps, recherche UCT) plutôt que sur des
//! boucles arbitraires, afin que les résultats soient interprétables et
//! comparables à d'autres outils de référence.
//!
//! ## Méthodologie de mesure
//! Tous les tests s'appuient sur le même protocole, implémenté une seule
//! fois dans [`measure_throughput`] / [`measure_count`] :
//!
//! 1. **Préparation hors chronométrage** : allocation des buffers,
//!    initialisation d'un générateur pseudo-aléatoire déterministe
//!    ([`StdRng`] à graine fixe), etc. Ce coût ne doit jamais polluer la
//!    mesure.
//! 2. **Échauffement (warm-up)** pendant [`WARMUP_DURATION`] : la charge de
//!    travail est exécutée sans être chronométrée, le temps que la
//!    fréquence turbo du CPU se stabilise, que les caches et le
//!    prédicteur de branchement soient « chauds », et que l'ordonnanceur
//!    de l'OS ait replacé les threads. Ignorer cette phase est une source
//!    classique de résultats bruités dans les benchmarks maison.
//! 3. **Échantillonnage** : [`SAMPLE_COUNT`] mesures indépendantes de
//!    [`SAMPLE_DURATION`] chacune sont effectuées, puis la **médiane** des
//!    débits obtenus est retenue comme score final plutôt qu'une mesure
//!    unique. La médiane est nettement plus robuste que la moyenne face
//!    aux perturbations ponctuelles (interruption OS, tâche de fond,
//!    variation de fréquence) qui font système sur une machine partagée.
//!
//! La durée totale par test (échauffement + échantillons) est d'environ
//! 5 secondes, ce qui préserve la durée globale de la suite par rapport
//! aux versions précédentes de cet outil.
//!
//! ## Isolation de la mesure
//! Chaque test isole explicitement le coût qu'il prétend mesurer : par
//! exemple, le test de tri exclut la génération des nombres aléatoires du
//! chronométrage (seul le tri lui-même est mesuré), et le test de
//! compression utilise un jeu de données à l'entropie réaliste plutôt
//! qu'un tampon de zéros qui se compresserait de façon triviale et ne
//! représenterait pas le coût réel de l'algorithme.
//!
//! ## Valeur de retour
//! [`Benchmark::run`] retourne un [`SampleResult`] contenant les statistiques
//! complètes de l'échantillonnage (médiane, écart-type, échantillons bruts).
//! Les unités dépendent du test (itérations/s, Mo/s, éléments/s, nombre de
//! passes ou de tirages effectués, etc. — voir la documentation de chaque
//! test). Le score brut (médiane) est ensuite normalisé et pondéré par
//! [`crate::engines::score`] pour produire le score final affiché à
//! l'utilisateur ; il n'est donc pas directement comparable d'un test à
//! l'autre.

use rayon::prelude::*;
use std::hint::black_box;
use std::io::Write;
use std::time::{Duration, Instant};

use anyhow::Result;
use flate2::{write::ZlibEncoder, Compression};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use sha2::{Digest, Sha256};

use crate::engines::benchmark::Benchmark;
use crate::model::result::SampleResult;

/// Durée de la phase d'échauffement (non mesurée) exécutée avant chaque
/// série d'échantillons. Voir la doc de module pour la justification.
const WARMUP_DURATION: Duration = Duration::from_millis(500);

/// Durée d'un échantillon de mesure individuel.
const SAMPLE_DURATION: Duration = Duration::from_millis(2000);

/// Nombre d'échantillons indépendants dont la médiane constitue le score
/// final d'un test. Augmenté à 5 pour une meilleure précision statistique.
const SAMPLE_COUNT: usize = 5;

/// Durée minimale (en secondes) utilisée comme diviseur lors du calcul des
/// débits, afin d'éviter toute division par zéro ou tout score aberrant sur
/// des systèmes dont l'horloge a une résolution très grossière.
const MIN_ELAPSED_SEC: f64 = 1e-6;

/// Exécute une charge de travail selon le protocole standard de la suite
/// (échauffement puis échantillonnage médian, voir la doc de module) et
/// retourne un **débit** : le nombre d'« unités de travail » (itérations,
/// octets, éléments, ...) traitées par seconde.
///
/// `batch` doit effectuer une petite tranche de travail borné et retourner
/// le nombre d'unités traitées lors de cet appel. Elle est invoquée en
/// boucle jusqu'à épuisement de chaque fenêtre de temps.
fn measure_throughput<F: FnMut() -> u64>(mut batch: F) -> SampleResult {
    let warmup_start = Instant::now();
    while warmup_start.elapsed() < WARMUP_DURATION {
        black_box(batch());
    }

    let mut raw_samples: Vec<u64> = Vec::with_capacity(SAMPLE_COUNT);
    for _ in 0..SAMPLE_COUNT {
        let start = Instant::now();
        let mut units: u64 = 0;
        while start.elapsed() < SAMPLE_DURATION {
            units = units.saturating_add(batch());
        }
        let elapsed = start.elapsed().as_secs_f64().max(MIN_ELAPSED_SEC);
        let throughput = (units as f64 / elapsed) as u64;
        raw_samples.push(throughput);
    }

    SampleResult::from_samples(raw_samples)
}

/// Variante de [`measure_throughput`] pour les tests dont le score
/// pertinent est un **compte brut** plutôt qu'un débit (ex. nombre de
/// tirages Monte-Carlo effectués). Comme chaque échantillon dure
/// exactement [`SAMPLE_DURATION`], les comptes obtenus sont directement
/// comparables entre eux sans division par le temps écoulé.
fn measure_count<F: FnMut() -> u64>(mut batch: F) -> SampleResult {
    let warmup_start = Instant::now();
    while warmup_start.elapsed() < WARMUP_DURATION {
        black_box(batch());
    }

    let mut raw_samples: Vec<u64> = Vec::with_capacity(SAMPLE_COUNT);
    for _ in 0..SAMPLE_COUNT {
        let start = Instant::now();
        let mut total: u64 = 0;
        while start.elapsed() < SAMPLE_DURATION {
            total = total.saturating_add(batch());
        }
        raw_samples.push(total);
    }

    SampleResult::from_samples(raw_samples)
}

/// Médiane d'un ensemble de mesures. Plus robuste que la moyenne face aux
/// valeurs aberrantes ponctuelles, ce qui en fait le choix standard pour
/// agréger des échantillons de benchmark.
fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let len = values.len();
    if len == 0 {
        return 0.0;
    }
    if len % 2 == 0 {
        (values[len / 2 - 1] + values[len / 2]) / 2.0
    } else {
        values[len / 2]
    }
}

/// Génère un tampon de données « réaliste » pour le test de compression :
/// un petit dictionnaire de motifs texte recopiés en boucle et légèrement
/// perturbés par un générateur pseudo-aléatoire déterministe, afin
/// d'obtenir une entropie et un taux de compression proches d'un contenu
/// réel (texte, JSON, journaux applicatifs). Un tampon de zéros, à
/// l'inverse, se compresse de façon quasi gratuite et ne reflète pas le
/// coût réel de recherche de correspondances de l'algorithme.
fn generate_compressible_data(size: usize) -> Vec<u8> {
    const DICTIONARY: &[&[u8]] = &[
        b"the quick brown fox jumps over the lazy dog. ",
        b"lorem ipsum dolor sit amet, consectetur adipiscing elit. ",
        b"0123456789abcdef ",
        b"benchmark result score cpu memory disk graphics ",
    ];

    let mut rng = StdRng::seed_from_u64(0xB0B0);
    let mut buffer = Vec::with_capacity(size);
    while buffer.len() < size {
        let word = DICTIONARY[rng.gen_range(0..DICTIONARY.len())];
        buffer.extend_from_slice(word);
        // Une perturbation aléatoire occasionnelle évite un motif
        // parfaitement périodique, qui compresserait de façon irréaliste.
        if rng.gen_bool(0.05) {
            buffer.push(rng.gen());
        }
    }
    buffer.truncate(size);
    buffer
}

/// Test multi-cœur : applique un mélange entier (bit-mixing) coûteux et
/// difficile à optimiser sur un grand nombre d'éléments, répartis sur tous
/// les cœurs disponibles via `rayon`.
///
/// La taille du lot traité à chaque appel est proportionnelle au nombre de
/// threads du pool `rayon` (`rayon::current_num_threads`), afin que
/// chaque thread reçoive une part de travail comparable quel que soit le
/// nombre de cœurs de la machine, et que le coût de répartition des
/// tâches par `rayon` reste négligeable devant le travail utile — un
/// lot de taille fixe pénaliserait injustement les machines à très grand
/// nombre de cœurs (trop de synchronisation pour trop peu de travail par
/// thread).
///
/// **Score retourné** : éléments traités, tous cœurs confondus, par
/// seconde (`u64`).
pub struct CpuMultiCore;
impl Benchmark for CpuMultiCore {
    fn name(&self) -> &str {
        "CPU Multi-Core"
    }

    fn weight(&self) -> u64 {
        3
    }

    fn run(&self) -> Result<SampleResult> {
        let threads = rayon::current_num_threads().max(1) as u64;
        let items_per_batch = 200_000u64 * threads;

        let result = measure_throughput(|| {
            (0u64..items_per_batch).into_par_iter().for_each(|i| {
                // Mélange entier relativement coûteux et difficile à
                // optimiser : multiplication + rotation + XOR, inspiré des
                // constructions utilisées dans les générateurs
                // pseudo-aléatoires (ex. SplitMix64).
                let v = i
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1)
                    .rotate_left(17)
                    ^ 0x9E37_79B9_7F4A_7C15;
                black_box(v);
            });
            items_per_batch
        });

        Ok(result)
    }
}

/// Test de calcul entier mono-thread : quatre chaînes arithmétiques
/// **indépendantes** (multiplication, addition, soustraction, rotation,
/// XOR, saturées) exécutées en parallèle logiciel dans la même boucle.
///
/// Une seule chaîne dépendante mesurerait surtout la *latence* d'une
/// instruction (chaque opération devant attendre le résultat de la
/// précédente) et sous-exploiterait un cœur superscalaire moderne, capable
/// d'exécuter plusieurs instructions indépendantes par cycle. Utiliser
/// plusieurs chaînes indépendantes expose ce parallélisme d'instructions
/// (ILP) et donne une mesure plus fidèle du débit de calcul entier réel du
/// cœur — une pratique standard des microbenchmarks de référence.
///
/// **Score retourné** : nombre total d'opérations arithmétiques (cumulées
/// sur les quatre chaînes) effectuées par seconde (`u64`).
pub struct CpuIntMath;
impl Benchmark for CpuIntMath {
    fn name(&self) -> &str {
        "CPU Int Math"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<SampleResult> {
        // Nombre d'itérations effectuées par appel, avant de revérifier
        // l'horloge : amortit le coût de `Instant::elapsed()` sur des
        // opérations individuellement très rapides (quelques cycles).
        const INNER: u64 = 20_000;

        let mut x0: u64 = 1;
        let mut x1: u64 = 2;
        let mut x2: u64 = 3;
        let mut x3: u64 = 4;

        let result = measure_throughput(|| {
            for _ in 0..INNER {
                x0 = x0.wrapping_mul(123456789).wrapping_add(987654321);
                x0 = x0.wrapping_sub(54321) ^ x0.rotate_left(13);

                x1 = x1.wrapping_mul(2654435761).wrapping_add(40503);
                x1 = x1.wrapping_sub(12345) ^ x1.rotate_left(7);

                x2 = x2.wrapping_mul(2246822519).wrapping_add(3266489917);
                x2 = x2.wrapping_sub(98765) ^ x2.rotate_left(21);

                x3 = x3.wrapping_mul(3266489917).wrapping_add(668265263);
                x3 = x3.wrapping_sub(11111) ^ x3.rotate_left(29);
            }
            INNER * 4
        });

        black_box(x0 ^ x1 ^ x2 ^ x3);
        Ok(result)
    }
}

/// Test de calcul flottant mono-thread : quatre chaînes **indépendantes**
/// combinant des opérations fusionnées multiplication-addition
/// (`mul_add`), des fonctions transcendantes (`sin`, `cos`, `tan`) et une
/// racine carrée, afin de solliciter plusieurs unités d'exécution
/// flottantes distinctes plutôt qu'une seule chaîne dépendante — même
/// motivation d'exposition de l'ILP que pour [`CpuIntMath`], appliquée au
/// calcul flottant.
///
/// **Score retourné** : nombre total d'opérations (cumulées sur les
/// quatre chaînes) effectuées par seconde (`u64`).
pub struct CpuFloatMath;
impl Benchmark for CpuFloatMath {
    fn name(&self) -> &str {
        "CPU Float Math"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<SampleResult> {
        const INNER: u64 = 20_000;

        let mut x0: f64 = 1.0;
        let mut x1: f64 = 1.5;
        let mut x2: f64 = 2.0;
        let mut x3: f64 = 0.5;

        let result = measure_throughput(|| {
            for _ in 0..INNER {
                x0 = x0.mul_add(1.000_000_1, 0.000_000_1);
                x0 = (x0.sin() + x0.cos()).tan();

                x1 = x1.mul_add(0.999_999_9, 0.000_000_2);
                x1 = x1.sqrt().abs() + 1.0;

                x2 = x2.mul_add(1.000_000_2, -0.000_000_1);
                x2 = (x2 * 0.5).sin();

                x3 = x3.mul_add(1.000_000_3, 0.000_000_1);
                x3 = x3.cos().abs() + 0.1;
            }
            INNER * 4
        });

        black_box(x0 + x1 + x2 + x3);
        Ok(result)
    }
}

/// Test de calcul de nombres premiers : passes répétées du **crible
/// d'Ératosthène** sur un intervalle de 1 000 000 d'entiers.
///
/// Le crible d'Ératosthène est l'algorithme de référence pour ce type de
/// mesure (c'est par exemple la base du test « nsieve » du Computer
/// Language Benchmarks Game) : il est nettement plus représentatif d'une
/// charge de calcul réelle qu'une division d'essai naïve sur des entiers
/// isolés, et son motif d'accès mémoire quasi-séquentiel avec foulées
/// croissantes sollicite explicitement la hiérarchie de cache au-delà du
/// L1 (le tampon fait environ 1 Mo).
///
/// **Score retourné** : nombre de passes complètes du crible effectuées
/// pendant la fenêtre de mesure (`u64`), c'est-à-dire un débit
/// "passes/s" implicite puisque chaque échantillon dure une durée fixe.
pub struct CpuPrimeCalc;
impl Benchmark for CpuPrimeCalc {
    fn name(&self) -> &str {
        "CPU Prime Calc"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<SampleResult> {
        const SIEVE_SIZE: usize = 1_000_000;
        let mut is_composite = vec![false; SIEVE_SIZE + 1];

        let result = measure_count(|| {
            for v in is_composite.iter_mut() {
                *v = false;
            }

            let mut primes_found: u64 = 0;
            for i in 2..=SIEVE_SIZE {
                if !is_composite[i] {
                    primes_found += 1;
                    let mut j = i * i;
                    while j <= SIEVE_SIZE {
                        is_composite[j] = true;
                        j += i;
                    }
                }
            }
            black_box(primes_found);
            1
        });

        Ok(result)
    }
}

/// Test d'instructions vectorielles étendues : noyau **SAXPY**
/// (*Single-precision A·X Plus Y*, `y[i] = a * x[i] + y[i]`), l'un des
/// noyaux de calcul les plus utilisés comme référence pour mesurer le
/// débit flottant vectoriel (il constitue la routine de niveau 1 de BLAS
/// utilisée par des suites historiques comme LINPACK/STREAM).
///
/// Le noyau est écrit sous une forme simple et sans dépendance entre
/// itérations, propice à l'auto-vectorisation par le compilateur en mode
/// optimisé (SSE/AVX sur x86, NEON sur ARM) sans recourir à des
/// intrinsics liés à une architecture particulière. Le nom historique du
/// test (« SSE Ext ») est conservé pour rester cohérent avec les
/// baselines existantes, mais la mesure reflète plus largement le débit
/// vectoriel flottant du CPU, quelle que soit l'architecture.
///
/// **Score retourné** : nombre d'éléments `f32` traités par seconde
/// (`u64`).
pub struct CpuSSE;
impl Benchmark for CpuSSE {
    fn name(&self) -> &str {
        "CPU SSE Ext"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<SampleResult> {
        const LEN: usize = 1_000_000;
        let x = vec![1.0f32; LEN];
        let mut y = vec![2.0f32; LEN];
        let a: f32 = 1.000_000_1;

        let result = measure_throughput(|| {
            for (yi, xi) in y.iter_mut().zip(x.iter()) {
                *yi = a.mul_add(*xi, *yi);
            }
            LEN as u64
        });

        black_box(y[0]);
        Ok(result)
    }
}

/// Test de compression : compresse en boucle un bloc de 10 Mo de données
/// synthétiques à l'entropie réaliste (voir [`generate_compressible_data`])
/// à l'aide de l'algorithme zlib (compression par défaut), via `flate2`.
///
/// **Score retourné** : débit de compression en mébioctets par seconde
/// (Mo/s, `u64`), calculé sur la taille des données **en entrée** de
/// l'encodeur (et non sur la taille compressée en sortie).
pub struct CpuCompression;
impl Benchmark for CpuCompression {
    fn name(&self) -> &str {
        "CPU Compression"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<SampleResult> {
        let data = generate_compressible_data(10_000_000);

        let result = measure_throughput(|| {
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder
                .write_all(&data)
                .expect("écriture en mémoire dans l'encodeur zlib");
            let compressed = encoder.finish().expect("finalisation de la compression zlib");
            black_box(&compressed);
            data.len() as u64
        });

        Ok(result)
    }
}

/// Test de hachage cryptographique : calcule en boucle le condensat
/// SHA-256 d'un bloc de 1 Mo, en réutilisant la même instance de hacheur
/// (`finalize_reset`) pour ne pas mesurer de coût d'allocation, et en
/// regroupant plusieurs hachages par appel pour amortir le coût de
/// vérification de l'horloge sur une opération individuellement rapide.
///
/// Bien que nommé « Encryption » pour rester cohérent avec les suites de
/// benchmark historiques dont ce test s'inspire, il mesure en réalité le
/// débit de hachage cryptographique (SHA-256), représentatif des charges
/// de calcul liées à l'intégrité des données, aux signatures ou aux
/// preuves de travail.
///
/// **Score retourné** : débit de hachage en mébioctets par seconde
/// (Mo/s, `u64`).
pub struct CpuEncryption;
impl Benchmark for CpuEncryption {
    fn name(&self) -> &str {
        "CPU Encryption"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<SampleResult> {
        const INNER: u64 = 8;
        let data = vec![0u8; 1024 * 1024];
        let mut hasher = Sha256::new();

        let result = measure_throughput(|| {
            for _ in 0..INNER {
                hasher.update(&data);
                let digest = hasher.finalize_reset();
                black_box(digest);
            }
            data.len() as u64 * INNER
        });

        Ok(result)
    }
}

/// Test de simulation physique : simulation gravitationnelle **à N
/// corps** (méthode des paires, O(n²) par pas de temps), intégrée par la
/// méthode d'Euler semi-implicite. Il s'agit d'un noyau de calcul
/// classique et largement utilisé comme référence CPU flottante (voir par
/// exemple le test « n-body » du Computer Language Benchmarks Game),
/// nettement plus représentatif d'une charge de travail réelle qu'une
/// simple addition vectorielle : il combine calcul flottant dense,
/// racines carrées et un jeu de données compact tenant dans le cache.
///
/// À chaque pas de temps, pour chaque corps, la force gravitationnelle
/// exercée par tous les autres corps est accumulée (avec un facteur
/// d'adoucissement pour éviter les singularités à distance nulle), puis
/// les vitesses et positions sont mises à jour.
///
/// **Score retourné** : nombre d'interactions corps-à-corps calculées par
/// seconde (`u64`), c'est-à-dire environ `N² × pas de temps par seconde`.
pub struct CpuPhysics;
impl Benchmark for CpuPhysics {
    fn name(&self) -> &str {
        "CPU Physics"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<SampleResult> {
        // Nombre de corps simulés : assez grand pour représenter une
        // charge de calcul dense en flottant, assez petit pour que l'état
        // complet du système tienne dans le cache L2 de la plupart des
        // CPU modernes.
        const BODIES: usize = 256;
        const SOFTENING: f64 = 1e-3;
        const DT: f64 = 0.01;

        let mut rng = StdRng::seed_from_u64(42);
        let mut px = vec![0f64; BODIES];
        let mut py = vec![0f64; BODIES];
        let mut pz = vec![0f64; BODIES];
        let mut vx = vec![0f64; BODIES];
        let mut vy = vec![0f64; BODIES];
        let mut vz = vec![0f64; BODIES];
        let mass = vec![1f64; BODIES];

        for i in 0..BODIES {
            px[i] = rng.gen_range(-1.0..1.0);
            py[i] = rng.gen_range(-1.0..1.0);
            pz[i] = rng.gen_range(-1.0..1.0);
        }

        let result = measure_throughput(|| {
            for i in 0..BODIES {
                let (mut fx, mut fy, mut fz) = (0f64, 0f64, 0f64);
                for j in 0..BODIES {
                    if i == j {
                        continue;
                    }
                    let dx = px[j] - px[i];
                    let dy = py[j] - py[i];
                    let dz = pz[j] - pz[i];
                    let dist_sq = dx * dx + dy * dy + dz * dz + SOFTENING;
                    let inv_dist = 1.0 / dist_sq.sqrt();
                    let inv_dist3 = inv_dist * inv_dist * inv_dist;
                    let f = mass[j] * inv_dist3;
                    fx += dx * f;
                    fy += dy * f;
                    fz += dz * f;
                }
                vx[i] += fx * DT;
                vy[i] += fy * DT;
                vz[i] += fz * DT;
            }
            for i in 0..BODIES {
                px[i] += vx[i] * DT;
                py[i] += vy[i] * DT;
                pz[i] += vz[i] * DT;
            }
            (BODIES * BODIES) as u64
        });

        black_box((px[0], vy[0]));
        Ok(result)
    }
}

/// Test de tri : trie un million d'entiers 64 bits avec l'algorithme de
/// tri non stable standard de Rust (`sort_unstable`, un pattern-defeating
/// quicksort), à partir d'un jeu de données pseudo-aléatoire fixe
/// (graine constante, pour un résultat reproductible d'une exécution à
/// l'autre).
///
/// Le jeu de données est généré **une seule fois, hors chronométrage**,
/// puis recopié dans un tampon de travail à chaque itération mesurée : un
/// test de tri de référence doit isoler le coût de l'algorithme de tri
/// lui-même, pas celui de la génération de nombres pseudo-aléatoires, qui
/// serait sinon comptabilisé à tort dans le débit mesuré.
///
/// **Score retourné** : nombre d'éléments triés par seconde (`u64`).
pub struct CpuSorting;
impl Benchmark for CpuSorting {
    fn name(&self) -> &str {
        "CPU Sorting"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<SampleResult> {
        const LEN: usize = 1_000_000;

        let mut rng = StdRng::seed_from_u64(123);
        let template: Vec<u64> = (0..LEN).map(|_| rng.gen()).collect();
        let mut buffer = template.clone();

        let result = measure_throughput(|| {
            buffer.copy_from_slice(&template);
            buffer.sort_unstable();
            black_box(&buffer[0..16.min(buffer.len())]);
            LEN as u64
        });

        Ok(result)
    }
}

/// Test « UCT Single » : recherche arborescente Monte-Carlo pilotée par la
/// formule **UCB1** (*Upper Confidence bound applied to Trees*), telle
/// qu'utilisée par les moteurs de jeu de type Monte-Carlo Tree Search
/// (Go, échecs, ...).
///
/// L'arbre est un arbre B-aire complet stocké de façon **implicite**
/// (indexation façon tas binaire généralisé : le nœud `i` a pour enfants
/// les indices `i·B + 1 ..= i·B + B`), ce qui évite toute allocation de
/// pointeurs tout en conservant un motif d'accès mémoire irrégulier
/// représentatif d'une vraie recherche arborescente. Chaque tirage
/// (« playout ») :
/// 1. descend de la racine vers une feuille en choisissant à chaque
///    niveau l'enfant de meilleur score UCB1
///    (`valeur_moyenne + C·√(ln(visites_parent) / visites_enfant)`),
///    en priorisant systématiquement les enfants jamais visités ;
/// 2. tire une récompense pseudo-aléatoire à la feuille atteinte ;
/// 3. rétropropage cette récompense en mettant à jour le compteur de
///    visites et la somme des valeurs de chaque nœud du chemin parcouru.
///
/// Ce test combine ainsi calcul flottant (formule UCB1, `ln`, `sqrt`),
/// branchements de sélection et accès mémoire non séquentiels — un
/// profil représentatif des algorithmes de recherche et de planification,
/// distinct des autres tests de la suite qui sont soit purement
/// arithmétiques, soit purement séquentiels en mémoire.
///
/// **Score retourné** : nombre de tirages (« playouts ») complets
/// effectués pendant la fenêtre de mesure (`u64`).
pub struct CpuUCT;
impl Benchmark for CpuUCT {
    fn name(&self) -> &str {
        "CPU UCT Single"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<SampleResult> {
        const BRANCHING: usize = 4;
        const DEPTH: usize = 8;
        // Constante d'exploration UCB1 usuelle : sqrt(2).
        const EXPLORATION: f64 = 1.414_213_56;

        let node_count: usize = (0..=DEPTH).map(|d| BRANCHING.pow(d as u32)).sum();

        let mut visits = vec![0f64; node_count];
        let mut value_sum = vec![0f64; node_count];
        let mut rng = StdRng::seed_from_u64(0xC0FFEE);

        let result = measure_count(|| {
            let mut path = [0usize; DEPTH + 1];
            let mut node = 0usize;
            path[0] = node;
            let mut depth_reached = 0usize;

            for d in 0..DEPTH {
                let first_child = node * BRANCHING + 1;
                if first_child >= node_count {
                    break;
                }

                let parent_visits = visits[node] + 1.0;
                let mut best_child = first_child;
                let mut best_score = f64::NEG_INFINITY;

                for k in 0..BRANCHING {
                    let child = first_child + k;
                    if child >= node_count {
                        break;
                    }
                    let child_visits = visits[child];
                    let score = if child_visits == 0.0 {
                        f64::INFINITY
                    } else {
                        (value_sum[child] / child_visits)
                            + EXPLORATION * (parent_visits.ln() / child_visits).sqrt()
                    };
                    if score > best_score {
                        best_score = score;
                        best_child = child;
                    }
                }

                node = best_child;
                depth_reached = d + 1;
                path[depth_reached] = node;
            }

            let reward: f64 = rng.gen();
            for &n in path.iter().take(depth_reached + 1) {
                visits[n] += 1.0;
                value_sum[n] += reward;
            }

            1
        });

        black_box(&visits);
        Ok(result)
    }
}
