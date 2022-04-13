//#![windows_subsystem = "windows"]
use iced::futures::io::Window;
use iced::{Element, Sandbox, Settings, Text, Button, button, Column, Align, Size, Point, mouse, Length, text_input, Row, window, slider, Checkbox};
use iced::canvas::{self, Canvas, Cursor, Fill, Frame, Geometry, Path, Program, Event, event};
use iced::{Color, Rectangle};
use iced::text_input::{TextInput};

use image::{ImageBuffer, Pixel, RgbImage};

use rand::{Rng, thread_rng, random};

use std::str::FromStr;
use std::io;
use std::fs::File;
use std::fs::create_dir;
use std::io::prelude::*;
use std::io::BufReader;

use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

const TAILLE_CASE: usize = 20;
const MATRICES_ROTATION:[(i32,i32,i32,i32);4] = [
    (1, 0,
     0, 1),
    (0,-1,
     1, 0),
    (-1, 0,
     0, -1),
    (0, 1,
     -1, 0), 
];

#[derive(Clone,Copy)]
struct Case {
    rempli:bool,
}

impl Case {
    fn new(rempli:bool) -> Case {
        Case {rempli}
    }
    fn est_rempli(&self) -> bool {
        self.rempli
    }
}
#[derive(Clone,Copy)]
struct Point2D {
    x:f32,
    y:f32,
    orig_x:f32,
    orig_y:f32,
}
impl Point2D {
    fn new(x:f32, y:f32) -> Point2D {
        Point2D {x, y, orig_x:x, orig_y:y}
    }
    fn rotation(&mut self, ang:f32){
        self.x = (self.orig_x * ang.cos() - self.orig_y * ang.sin()).round();
        self.y = (self.orig_y * ang.cos() + self.orig_x * ang.sin()).round();
    }
    fn co_entiers(&self) -> (i32, i32) {
        (self.x as i32, self.y as i32)
    }
}
#[derive(Clone,Copy)]
struct Point2Di {
    x:i32,
    y:i32,
    orig_x:i32,
    orig_y:i32,
}
impl Point2Di {
    fn new(x:i32, y:i32) -> Point2Di {
        Point2Di {x, y, orig_x:x, orig_y:y}
    }
    fn rotation(&mut self, ang:usize){
        self.x = self.orig_x * MATRICES_ROTATION[ang].0 + self.orig_y * MATRICES_ROTATION[ang].1;
        self.y = self.orig_x * MATRICES_ROTATION[ang].2 + self.orig_y * MATRICES_ROTATION[ang].3;
    }
    fn co_entiers(&self) -> (i32, i32) {
        (self.x, self.y)
    }
}

struct Piece {
    positions:Vec<Point2Di>,
    orientation:u8,
    hauteur:u8,
    nom:String,
}

impl Piece {
    fn new(positions:Vec<Point2Di>, nom:String) -> Piece {
        let mut piece = Piece {positions, orientation:0, hauteur:0,nom};
        piece
    }
    fn calcul_hauteur(&mut self) {
        let mut plus_haut = 0;
        let mut plus_bas = 0;
        for point in &self.positions {
            if point.y < plus_bas {
                plus_bas = point.y
            }
            if point.y > plus_haut {
                plus_haut = point.y
            }
        }
        self.hauteur = (plus_haut - plus_bas) as u8;
    }
    fn rotation_precise(&mut self, nouvelle_orient:u8) {
        if nouvelle_orient >= 4 {
            self.orientation = 3;
        }
        else {
            self.orientation = nouvelle_orient;
        }
        
        self.rotation();
    }
    fn point_plus_bas(&self) -> Point2Di {
        let mut plus_bas = self.positions[0];
        for pos in &self.positions {
            if pos.y < plus_bas.y {
                plus_bas = *pos
            }
        }
        plus_bas
    }
    fn cote_bas(&self) -> Vec<Point2Di> {
        let mut bas = Vec::with_capacity(self.positions.len());
        for pos1 in &self.positions {
            let mut dessous = true;
            for pos2 in &self.positions {
                if pos2.y < pos1.y && pos2.x == pos1.x {
                    dessous = false;
                    break
                }
            }
            if dessous {
                bas.push(*pos1);
            }
        }
        bas
    }
    fn rotation(&mut self) {
        for point in &mut self.positions {
            point.rotation(self.orientation as usize)
        }
        self.calcul_hauteur();
    }
    fn rotation_sens_aiguille(&mut self) {
        self.orientation += 1;
        if self.orientation > 3 {
            self.orientation = 0;
        }
        self.rotation();
    }
}

struct EtapeTetris {
    piece:usize,
    orientation:u8,
    colonne:usize,
    goto:Option<usize>,
}

struct PartieTetris {
    hauteur:usize,
    largeur:usize,
    deroulement:Vec<EtapeTetris>,
}

impl PartieTetris {
    fn new(hauteur:usize, largeur:usize) -> PartieTetris {
        PartieTetris{hauteur, largeur, deroulement:Vec::new()}
    }
    fn charger_depuis_fichier(nom:&str) -> Option<PartieTetris> {
        match File::open(format!("Parties/{}.txt",nom)) {
            Ok(fic) => {
                let reader = BufReader::new(fic);
                let mut lignes = reader.lines();
                let ligne_1 = lignes.next().unwrap().unwrap();
                let haut_larg: Vec<&str> = ligne_1.split_whitespace().collect();
                let (hauteur, largeur) = (usize::from_str(&haut_larg[0]).unwrap(), usize::from_str(&haut_larg[1]).unwrap());
                let mut etapes = Vec::<EtapeTetris>::new();
                for ligne in lignes {
                    let ligne_str = ligne.unwrap();
                    let composants: Vec::<&str> = ligne_str.split_whitespace().collect();
                    let mut goto = None;
                    if composants.len() == 4 {
                        goto = Some(usize::from_str(&composants[3]).unwrap());
                    }
                    etapes.push( EtapeTetris {
                        piece:usize::from_str(&composants[0]).unwrap(),
                        orientation:u8::from_str(&composants[1]).unwrap(),
                        colonne:usize::from_str(&composants[2]).unwrap(),
                        goto
                    }
                    )
                }
                return Some(PartieTetris{hauteur, largeur, deroulement:etapes})
                
                
            },
            Err(err) => { return None }
        }
    }
    fn sauvegarde_fichier_partie(&self, nom:&str) {
        match File::create(format!("Parties/{}.txt", nom)) {
            Ok(fic) => {
                let mut scripteur = fic;
                writeln!(&mut scripteur,"{} {}", self.hauteur, self.largeur).unwrap();
                for etape in &self.deroulement {
                    match etape.goto {
                        Some(destination) => {
                            writeln!(&mut scripteur,"{} {} {} {}", etape.piece, etape.orientation, etape.colonne, destination).unwrap();
                        }
                        None => {
                            writeln!(&mut scripteur,"{} {} {}", etape.piece, etape.orientation, etape.colonne).unwrap();
                        }
                    }
                    
                }
            },
            Err(_err) => println!("nom invalide ou dossier inexistant!")
        }
    }
}

struct Tableau {
    hauteur:usize,
    largeur:usize,
    h_i32:i32,
    l_i32:i32,
    tableau:Vec<Case>,
    case_pleine:Case,
    case_vide:Case,
}

impl Tableau {
    fn new(hauteur:usize, largeur:usize) -> Tableau {
        Tableau { hauteur, largeur,h_i32:hauteur as i32,l_i32:largeur as i32, tableau:vec![Case::new(false);hauteur*largeur], case_pleine:Case::new(true), case_vide:Case::new(false) }
    }
    fn dans(&self, x:i32, y:i32) -> bool {
        if self.h_i32 > y && y > -1 && self.l_i32 > x && x > -1 {
            true
        }
        else {
            false
        }
    }
    fn case_a(&self, x:i32, y:i32) -> &Case {
        if self.dans(x,y) {
            &self.tableau[y as usize * self.largeur + x as usize]
        }
        else if y >= self.h_i32 && -1 < x && x < self.l_i32 {
            &self.case_vide
        }
        else {
            &self.case_pleine
        }
    }
    fn rempli_a(&self, x:i32, y:i32) -> bool {
        self.case_a(x, y).est_rempli()
    }

    fn tient_dans(&self,x:i32, y:i32, piece:&Piece) -> bool {
        for position in &piece.positions {
            let (x_pos,y_pos) = position.co_entiers();
            if self.rempli_a(x_pos + x, y_pos + y) {
                return false;
            }
        }
        true
    }
    fn rajoute_piece(&mut self, x:i32, y:i32,piece:&Piece) -> bool {
        if self.tient_dans(x, y, piece) {
            for position in &piece.positions {
                let (x_pos,y_pos) = position.co_entiers();
                if !self.dans(x + x_pos, y + y_pos) {
                    return false
                }
            }
            for position in &piece.positions {
                let (x_pos,y_pos) = position.co_entiers();
                self.tableau[(y_pos + y) as usize * self.largeur + (x_pos + x) as usize].rempli = true;
            }
            true
        }
        else {
            false
        }
    }

    fn check_sweep(&self) -> Option<usize> {
        for y in 0..self.hauteur {
            let mut a_sweep = true;
            for x in 0..self.largeur {
                if !self.rempli_a(x as i32, (self.hauteur - 1 - y) as i32) {
                    a_sweep = false;
                    break
                }
            }
            if a_sweep {
                return Some(self.hauteur - 1 - y)
            }
        }
        
        None
        
    }

    fn vider(&mut self) {
        for case in &mut self.tableau {
            case.rempli = false;
        }
    }

    fn fait_sweep(&mut self, y:usize) {
        for posy in y..self.hauteur {
            for x in 0..self.largeur {
                self.tableau[posy * self.largeur + x] = if posy < self.hauteur-1 {self.tableau[(posy+1) * self.largeur + x]} else {self.case_vide};
            }
        }
    }

    fn iteration_sweep(&mut self) -> Option<u64> {
        let mut check = self.check_sweep();
        let mut nombre_sweep = 0;
        while check != None {
            let altitude = check.as_ref()?;
            self.fait_sweep(*altitude);
            check = self.check_sweep();
            nombre_sweep += 1;
        }
        if nombre_sweep == 0 {
            Some(0)
        }
        else {
            Some(nombre_sweep)
        }
    }
    fn tombe_piece(&self, piece:&Piece,x:usize) -> Option<usize> {
        let mut plus_bas:usize = self.hauteur - piece.hauteur as usize;
        let mut max_recurs = 0;
        while max_recurs < 2*self.hauteur {
            if self.tient_dans(x as i32, plus_bas as i32, piece) {
                if plus_bas > 0 {
                    plus_bas -= 1;
                }
                else {
                    break
                }
            }
            else {
                plus_bas += 1;
                break
            }
            max_recurs += 1;
        }
        if plus_bas >= self.hauteur - piece.hauteur as usize {
            return None;
        }
        Some(plus_bas)
    }
    fn genere_accroches(&self, piece:&mut Piece) -> Vec<Accroche> {
        let mut accroches = Vec::<Accroche>::new(); // Colonne,orientation,altitude
        for x in 0..self.largeur {
            for orient in 0..4 {
                piece.rotation_precise(orient);
                match self.tombe_piece(piece, x) {
                    Some(alt) => accroches.push(Accroche{colonne:x,orientation:orient,altitude:alt}),
                    None => ()
                }
            }
        }
        return accroches
    }
    fn algo_X_vision(&mut self, mut pieces:Vec<Piece>, etape:i32, coefficients:&CoefAlgo) -> (Accroche, i32) {
        let mut piece = pieces.remove(0);
        let accroches = self.genere_accroches(&mut piece);
        if accroches.len() == 0 {return (Accroche{colonne:0, orientation:0, altitude:0}, -20000000) }
        if pieces.len() > 0 {
            let mut best_accroche = None;
            for accroche in accroches {
                piece.rotation_precise(accroche.orientation);
                let (mut tableau_clone, nombre_sweep) = self.genere_clone_plus_piece(&piece, &accroche);
                let mut pieces2 = Vec::with_capacity(pieces.len());
                for piece in &pieces {
                    let mut nouvpiece = Piece{orientation:piece.orientation, hauteur:piece.hauteur, positions:Vec::with_capacity(piece.positions.len()), nom:piece.nom.clone()};
                    for pos in &piece.positions {
                        nouvpiece.positions.push(*pos);
                    }
                    pieces2.push(nouvpiece);
                }
                let (accroche_possible,score_branche) = tableau_clone.algo_X_vision(pieces2, etape + 3, coefficients);
                match best_accroche {
                    None => {
                        let data_acc_pres = accroche.get_data(&mut piece, &self, nombre_sweep as i32, etape);
                        best_accroche = Some((accroche, data_acc_pres.get_score(coefficients), accroche_possible.get_data(&mut pieces[0], &tableau_clone, nombre_sweep as i32, etape), score_branche))
                    },
                    Some((accroche_pres, score_accroche_pres, data, score)) => {
                        let propre_data = accroche.get_data(&mut piece, &self, nombre_sweep as i32, etape);
                        if score_branche + propre_data.get_score(coefficients) > score + score_accroche_pres {
                            best_accroche = Some((accroche, propre_data.get_score(coefficients), accroche_possible.get_data(&mut pieces[0], &tableau_clone, nombre_sweep as i32, etape), score_branche));
                        }
                        else if score_branche + propre_data.get_score(coefficients) == score + score_accroche_pres {
                            if propre_data.altitude < data.altitude {
                                best_accroche = Some((accroche, propre_data.get_score(coefficients), accroche_possible.get_data(&mut pieces[0], &tableau_clone, nombre_sweep as i32, etape), score_branche));
                            }
                        }
                    }
                }
            }
            (best_accroche.unwrap().0, best_accroche.unwrap().3 + best_accroche.unwrap().2.get_score(coefficients))
        }
        else {
            let mut meilleure_accroche = accroches[0];
            let (tableau_clone, nombre_sweep) = self.genere_clone_plus_piece(&piece, &meilleure_accroche);
            
            for accroche in accroches {
                let (tableau_clone2, nombre_sweep2) = self.genere_clone_plus_piece(&piece, &accroche);
                if accroche.get_data(&mut piece, &self, nombre_sweep2 as i32, etape).compare_accroches(&meilleure_accroche.get_data(&mut piece, &self, nombre_sweep as i32, etape),coefficients) {
                    meilleure_accroche = accroche
                }
            }
            (meilleure_accroche, meilleure_accroche.get_data(&mut piece, &self, nombre_sweep as i32, etape).get_score(coefficients))
        }
    }
    fn genere_clone_plus_piece(&self,piece:&Piece, accroche:&Accroche) -> (Tableau, u64) {
        let mut out = Tableau::new(self.hauteur, self.largeur);
        for i in 0..self.tableau.len() {
            out.tableau[i] = self.tableau[i]
        }
        out.rajoute_piece(accroche.colonne as i32, accroche.altitude as i32, piece);
        let nombre_sweep = out.iteration_sweep().unwrap();
        (out, nombre_sweep)
    }
}

#[derive(Clone, Copy)]
struct CoefAlgo {
    rempdess:i32,
    rempsides:i32,
    alt:i32,
    nb_sweep:i32,
    tr_imp_c:i32,
}

#[derive(Clone, Copy)]
struct Accroche {
    colonne:usize,
    orientation:u8,
    altitude:usize,
}

impl Accroche {
    fn get_data(&self, piece:&mut Piece, tableau:&Tableau, sweep:i32, etape:i32) -> DataAccroche {
        let mut numrempdess = 0;
        let mut numrempsides = 0;
        let mut trous_imp_cree = 0;
        piece.rotation_precise(self.orientation);
        for position in &piece.positions {
            if tableau.rempli_a(self.colonne as i32 + position.x as i32, self.altitude as i32 + position.y as i32 - 1) {
                numrempdess += 1;
            }
            if tableau.rempli_a(self.colonne as i32 + position.x as i32 + 1, self.altitude as i32 + position.y as i32) {
                numrempsides += 1;
            }
            if tableau.rempli_a(self.colonne as i32 + position.x as i32 - 1, self.altitude as i32 + position.y as i32) {
                numrempsides += 1;
            }
        }
        let bas_piece = piece.cote_bas();
        for pos in bas_piece {
            let mut posy = pos.y as i32 - 1 + self.altitude as i32;
            while !tableau.rempli_a(self.colonne as i32, posy as i32) {
                trous_imp_cree += 1;
                posy -= 1;
            } 
        }
        DataAccroche {numrempdess, numrempsides, altitude:piece.point_plus_bas().y as i32 + self.altitude as i32, nombre_sweep:sweep, trous_imp_cree, etape}
    }
}
#[derive(Clone, Copy)]
struct DataAccroche {
    numrempdess:i32,
    numrempsides:i32,
    altitude:i32,
    trous_imp_cree:i32,
    nombre_sweep:i32,
    etape:i32,
}

impl DataAccroche {
    fn compare_accroches(&self, autre:&DataAccroche, coef:&CoefAlgo) -> bool { // true si self > autre
        let score_s = self.get_score(coef);
        let score_a = autre.get_score(coef);
        if score_s > score_a {
            true
        }
        else if score_s == score_a {
            if self.altitude > autre.altitude {
                false
            }
            else {
                true
            }
        }
        else {
            false
        }
    }
    fn get_score(&self, coef:&CoefAlgo) -> i32 {
        //(self.numrempdess * 12 + self.numrempsides * 8 - self.altitude * 2 + self.nombre_sweep * 9 - self.trous_imp_cree * 9) / self.etape
        (self.numrempdess * coef.rempdess + self.numrempsides * coef.rempsides - self.altitude * coef.alt + self.nombre_sweep * coef.nb_sweep - self.trous_imp_cree * coef.tr_imp_c) / self.etape
    } 
}

enum Interaction {
    DansTableau(f32, f32),
    Rien,
}

struct Moteur {
    jeu:Tableau,
    pieces:Vec<Piece>,
    partie:PartieTetris,
    piece_choisie:usize,
    interaction:Interaction,
    mode_jeu:bool,
    vision:(u16,Vec<u16>,CoefAlgo),
}

impl Moteur {
    fn new(hauteur:usize, largeur:usize,mode_jeu:bool) -> Moteur {
        let mut out = Moteur {
            jeu: Tableau::new(hauteur, largeur),
            pieces: Vec::new(),
            partie:PartieTetris::new(hauteur, largeur),
            piece_choisie:1,
            interaction:Interaction::Rien,
            mode_jeu,
            vision:(3, Vec::new(), 
            CoefAlgo {
                rempdess:35,
                rempsides:24,
                alt:24,
                nb_sweep:30,
                tr_imp_c:14,
            })
        };
        
        out
    }

    fn joue_n_coups(&mut self, n:u32, coefs:CoefAlgo) -> u32 {
        self.jeu.vider();
        self.partie.deroulement.clear();
        self.vision.1.clear();
        let mut rand = rand::thread_rng();
        for i in 0..self.vision.0 {
            self.vision.1.push(rand.gen_range(0..self.pieces.len()) as u16)
        }
        for i in 0..n {
            if !self.joue_algo_trouve(&coefs) {
                return i
            }
        }
        n
    }

    fn simulation_vision_n_coef(&mut self, vis:u16,botrange:CoefAlgo, toprange:CoefAlgo, maxcoups:u32, nb_simu:u32) ->  Vec<(CoefAlgo, u32)> {
        let mut rando = rand::thread_rng();
        let mut coeftestes = Vec::<(CoefAlgo, u32)>::with_capacity(nb_simu as usize);
        
        self.vision.0 = vis;
        let mut coef_act:CoefAlgo;
        for i in 0..nb_simu {
            coef_act = CoefAlgo {
                nb_sweep:rando.gen_range(botrange.nb_sweep..toprange.nb_sweep),
                tr_imp_c:rando.gen_range(botrange.tr_imp_c..toprange.tr_imp_c),
                rempdess:rando.gen_range(botrange.rempdess..toprange.rempdess),
                rempsides:rando.gen_range(botrange.rempsides..toprange.rempsides),
                alt:rando.gen_range(botrange.alt..toprange.alt),
            };
            coeftestes.push((coef_act, self.joue_n_coups(maxcoups, coef_act)))
        }

        coeftestes.sort_by(|a,b| a.1.cmp(&b.1));
        coeftestes
    }
    fn simulation_vision_n_coef_solo(&mut self, vis:u16,botrange:CoefAlgo, toprange:CoefAlgo, maxcoups:u32, nb_simu:u32, thread:u32) ->  (CoefAlgo, u32) {
        let mut rando = rand::thread_rng();
        let mut best = (botrange, 0);
        self.vision.0 = vis;
        let mut coef_act:CoefAlgo;
        for i in 0..nb_simu {
            coef_act = CoefAlgo {
                nb_sweep:rando.gen_range(botrange.nb_sweep..toprange.nb_sweep),
                tr_imp_c:rando.gen_range(botrange.tr_imp_c..toprange.tr_imp_c),
                rempdess:rando.gen_range(botrange.rempdess..toprange.rempdess),
                rempsides:rando.gen_range(botrange.rempsides..toprange.rempsides),
                alt:rando.gen_range(botrange.alt..toprange.alt),
            };
            let score = self.joue_n_coups(maxcoups, coef_act);
            if score > best.1 {
                best = (coef_act, score)
            }
            //if (i+1) % 1 == 0 {
            //    println!("simulation {} du thread {} terminée", (i+1), thread);
            //}
            println!("simulation {} du thread {} terminée", i, thread);
        }

        best
    }

    fn joue_algo_trouve(&mut self, coefs:&CoefAlgo) -> bool {
        let mut pieces_out = Vec::<Piece>::new();
        for id in &self.vision.1 {
            let piece = &self.pieces[*id as usize];
            let mut nouvpiece = Piece{orientation:piece.orientation, hauteur:piece.hauteur, positions:Vec::new(), nom:piece.nom.clone()};
            for pos in &piece.positions {
                nouvpiece.positions.push(*pos);
            }
            pieces_out.push(nouvpiece);
        }
        let (play,score) = self.jeu.algo_X_vision(pieces_out, 1, coefs);
        if score <= -19000000 {
            return false
        }
        self.pieces[self.vision.1[0] as usize].rotation_precise(play.orientation);
        self.jeu.rajoute_piece(play.colonne as i32, play.altitude as i32, &self.pieces[self.vision.1[0] as usize]);
        self.nouvelle_etape(play.colonne, self.vision.1[0] as usize, play.orientation);
        self.vision.1.remove(0);
        let mut rand = rand::thread_rng();
        self.vision.1.push(rand.gen_range(0..self.pieces.len()) as u16);
        self.jeu.iteration_sweep();
        true
    }

    fn nouvelle_etape(&mut self, colonne:usize, piece:usize, orientation:u8) {
        self.partie.deroulement.push(EtapeTetris{colonne, piece, orientation, goto:None})
    }
    fn vider_tableau(&mut self) {
        self.jeu.vider();
    }
    fn pieces_standard(&mut self) {
        self.pieces.push(Piece::new(vec![Point2Di::new(0,0), Point2Di::new(-1,0), Point2Di::new(0, -1), Point2Di::new(-1, -1)], String::from("carré"))); //carré

        self.pieces.push(Piece::new(vec![Point2Di::new(1,0), Point2Di::new(0,0), Point2Di::new(-1, 0), Point2Di::new(-2, 0)], String::from("barre"))); // barre

        self.pieces.push(Piece::new(vec![Point2Di::new(0,0), Point2Di::new(1,0), Point2Di::new(0, -1), Point2Di::new(-1, -1)], String::from("éclair 1"))); // S 1
        self.pieces.push(Piece::new(vec![Point2Di::new(0,0), Point2Di::new(-1,0), Point2Di::new(0, -1), Point2Di::new(1, -1)], String::from("éclair 2"))); // S 2

        self.pieces.push(Piece::new(vec![Point2Di::new(0,0), Point2Di::new(1,0), Point2Di::new(-1, 0), Point2Di::new(0, 1)], String::from("pyramide"))); // pyramide
        
        self.pieces.push(Piece::new(vec![Point2Di::new(0,1), Point2Di::new(0,0), Point2Di::new(1, 0), Point2Di::new(2, 0)], String::from("L 1"))); // L 1
        self.pieces.push(Piece::new(vec![Point2Di::new(-2,0), Point2Di::new(-1,0), Point2Di::new(0, 1), Point2Di::new(0,0)], String::from("L 2"))); // L 2
    }
    fn intialisation(&mut self) {
        self.pieces_standard();
        let mut rand = rand::thread_rng();
        for i in 0..self.vision.0 {
            self.vision.1.push(rand.gen_range(0..self.pieces.len()) as u16)
        }
        self.jeu.vider();
    }
    fn joue_etape_imprime(&mut self,etape:usize, image:&mut Framebuffer, decalage_h:usize, decalage_v:usize) {
        let etape_partie = &self.partie.deroulement[etape];
        let mut tableau_avant = Vec::<Case>::new();
        for case in &self.jeu.tableau {
            tableau_avant.push(*case);
        }
        self.pieces[etape_partie.piece].rotation_precise(etape_partie.orientation);
        match self.jeu.tombe_piece(&self.pieces[etape_partie.piece], etape_partie.colonne) {
            Some(altitude) => {
                self.jeu.rajoute_piece(etape_partie.colonne as i32, altitude as i32, &self.pieces[etape_partie.piece]);
            },
            None => (),
        }
        for x in 0..self.jeu.largeur {
            for y in 0..self.jeu.hauteur {
                if !(tableau_avant[y * self.jeu.largeur + x].rempli == self.jeu.rempli_a(x as i32, y as i32)) {
                    image.rectangle((x * 10 + decalage_h, image.larg - y * 10 - decalage_v), (x * 10 + 10 + decalage_h, image.larg - y * 10 - decalage_v + 10), ColRGB::new(255, 0, 0))
                }
            }
        }

    }
    fn joue_etape(&mut self, etape:usize) {
        let etape_partie = &self.partie.deroulement[etape];
        self.pieces[etape_partie.piece].rotation_precise(etape_partie.orientation);
        match self.jeu.tombe_piece(&self.pieces[etape_partie.piece], etape_partie.colonne) {
            Some(altitude) => {
                self.jeu.rajoute_piece(etape_partie.colonne as i32, altitude as i32, &self.pieces[etape_partie.piece]);
            },
            None => (),
        }
        self.jeu.iteration_sweep();
    }
    fn boucle_joueur(&mut self) {
        self.jeu.iteration_sweep();
        let piece = 0;
        let orientation = 0;
        self.pieces[piece].rotation_precise(orientation);
        let colonne = 1;
        match self.jeu.tombe_piece(&self.pieces[piece], colonne) {
            Some(hauteur) => {
                match self.jeu.rajoute_piece(colonne as i32, hauteur as i32, &self.pieces[piece]) {
                    true => self.nouvelle_etape(colonne, piece, orientation),
                    false => ()
                }
            },
            None => (),
        }
        
    }
    fn sweep_tableau_imprime(&mut self, image: &mut Framebuffer, decalage_h:usize, decalage_v:usize) {
        'loopy:for y in 0..self.jeu.hauteur {
            for x in 0..self.jeu.largeur {
                if !self.jeu.rempli_a(x as i32, y as i32) {continue 'loopy}
                
            }
            image.ligne((decalage_h as f32, (image.larg - y * 10 - decalage_v + 5) as f32),((decalage_h + self.jeu.largeur * 10) as f32, (image.larg - y * 10 - decalage_v + 5) as f32),ColRGB::new(0, 0, 255), ColRGB::new(0, 0, 255));
            image.ligne((decalage_h as f32, (image.larg - y * 10 - decalage_v + 4) as f32),((decalage_h + self.jeu.largeur * 10) as f32, (image.larg - y * 10 - decalage_v + 4) as f32),ColRGB::new(0, 0, 255), ColRGB::new(0, 0, 255));
            image.ligne((decalage_h as f32, (image.larg - y * 10 - decalage_v + 6) as f32),((decalage_h + self.jeu.largeur * 10) as f32, (image.larg - y * 10 - decalage_v + 6) as f32),ColRGB::new(0, 0, 255), ColRGB::new(0, 0, 255));
        }
        self.jeu.iteration_sweep();
    }
    fn imprime_tableau(&self, decalage_h:usize, decalage_v:usize, image:&mut Framebuffer) {
        for y in 0..self.jeu.hauteur {
            for x in 0..self.jeu.largeur {
                image.ligne(((decalage_h + x * 10) as f32, (image.larg - y * 10 - decalage_v + 5) as f32),((decalage_h + x * 10) as f32, (image.larg - y * 10 - decalage_v + 5) as f32),ColRGB::new(128, 128, 128), ColRGB::new(128, 128, 128));
                if self.jeu.rempli_a(x as i32, y as i32) {
                    image.rectangle((x * 10 + decalage_h, image.larg - y * 10 - decalage_v), (x * 10 + 10 + decalage_h, image.larg - y * 10 - decalage_v + 10), ColRGB::new(0, 255, 0))
                }
                else {
                    image.rectangle((x * 10 + decalage_h, image.larg - y * 10 - decalage_v), (x * 10 + 10 + decalage_h, image.larg - y * 10 - decalage_v + 10), ColRGB::new(0, 0, 0))
                }
            }
        }
    }
    fn cree_infograph(&mut self) {
        let mut image = Framebuffer::new(60 + self.jeu.largeur * 10 * (self.partie.deroulement.len() + 2) + 20 * self.partie.deroulement.len(), self.jeu.hauteur * 10 + 20, ColRGB::new(255, 255, 255));
        self.vider_tableau();
        let mut decalage_h = 20;
        let decalage_v = 20;
        self.imprime_tableau(decalage_h, decalage_v, &mut image);
        image.pointe_horiz(((decalage_h + self.jeu.largeur * 10 + 1) as f32, (self.jeu.hauteur * 5 + decalage_v) as f32),((decalage_h + self.jeu.largeur * 10 + 20) as f32, (self.jeu.hauteur * 5 + decalage_v) as f32), 6,ColRGB::new(100, 100, 100), ColRGB::new(100, 100, 100));
        for etape in 0..self.partie.deroulement.len() {
            decalage_h += 20 + self.jeu.largeur * 10;
            
            self.imprime_tableau(decalage_h, decalage_v, &mut image);
            self.joue_etape_imprime(etape, &mut image, decalage_h, decalage_v);

            image.pointe_horiz(((decalage_h + self.jeu.largeur * 10 + 1) as f32, (self.jeu.hauteur * 5 + decalage_v) as f32),((decalage_h + self.jeu.largeur * 10 + 20) as f32, (self.jeu.hauteur * 5 + decalage_v) as f32), 6,ColRGB::new(100, 100, 100), ColRGB::new(100, 100, 100));
            
            self.sweep_tableau_imprime(&mut image, decalage_h, decalage_v);
        }
        decalage_h += 20 + self.jeu.largeur * 10;
        self.imprime_tableau(decalage_h, decalage_v, &mut image);
        let mut colbuf = Vec::<u8>::with_capacity(image.data.len() * 3);
        for col in image.data {
            colbuf.push(col.r);
            colbuf.push(col.g);
            colbuf.push(col.b);
        }
        let buffer = RgbImage::from_vec(image.long as u32, image.larg as u32, colbuf).unwrap();
        let nb:u32 = random();
        buffer.save(format!("Graphiques/{}.png", nb)).unwrap();

    }
}

#[derive(Debug, Clone)]
enum Message {
    PosePiece(usize),
    Rotation,
    ChangePiece,
    Sauvegarde,
    Charge,
    Rien,
    ChangeTexte(String),
    SliderLargeur(i32),
    SliderHauteur(i32),
    SliderVision(u16),
    SliderCoups(u16),
    ViderTableau,
    Rejouer,
    ChangeMode(bool),
    ChangePossPiece(bool,u32),
    FaitInfographique,
    JoueAuto
}

impl Default for InterfaceTetris {
    fn default() -> Self {
        let mut moteur = Moteur::new(20, 10, false);
        moteur.intialisation();

        let mut pieces_dispo = Vec::new();

        pieces_dispo.push((true,Piece::new(vec![Point2Di::new(0,0), Point2Di::new(-1,0), Point2Di::new(0, -1), Point2Di::new(-1, -1)], String::from("carré")))); //carré

        pieces_dispo.push((true,Piece::new(vec![Point2Di::new(1,0), Point2Di::new(0,0), Point2Di::new(-1, 0), Point2Di::new(-2, 0)], String::from("barre")))); // barre

        pieces_dispo.push((true,Piece::new(vec![Point2Di::new(0,0), Point2Di::new(1,0), Point2Di::new(0, -1), Point2Di::new(-1, -1)], String::from("éclair 1")))); // S 1
        pieces_dispo.push((true,Piece::new(vec![Point2Di::new(0,0), Point2Di::new(-1,0), Point2Di::new(0, -1), Point2Di::new(1, -1)], String::from("éclair 2")))); // S 2

        pieces_dispo.push((true,Piece::new(vec![Point2Di::new(0,0), Point2Di::new(1,0), Point2Di::new(-1, 0), Point2Di::new(0, 1)], String::from("pyramide")))); // pyramide
        
        pieces_dispo.push((true,Piece::new(vec![Point2Di::new(0,1), Point2Di::new(0,0), Point2Di::new(1, 0), Point2Di::new(2, 0)], String::from("L 1")))); // L 1
        pieces_dispo.push((true,Piece::new(vec![Point2Di::new(-2,0), Point2Di::new(-1,0), Point2Di::new(0, 1), Point2Di::new(0,0)], String::from("L 2")))); // L 2


        InterfaceTetris {
            bouton_rotation: button::State::default(),
            bouton_change: button::State::default(),
            bouton_sauvegarde: button::State::default(),
            bouton_charge: button::State::default(),
            entree_fichier: text_input::State::default(),
            slider_largeur: slider::State::default(),
            slider_hauteur: slider::State::default(),
            slider_vision: slider::State::default(),
            slider_coups: slider::State::default(),
            texte_fichier: String::from("DEFAULT"),
            bouton_vider: button::State::default(),
            bouton_rejoue: button::State::default(),
            bouton_info: button::State::default(),
            bouton_auto: button::State::default(),
            vision_choisie : 3,
            largeur_choisie : 10,
            hauteur_choisie : 20,
            coups_choisis : 1,
            moteur,
            etat_partie:EtatPartie::Normal,
            etape_actuelle:0,
            mode_jeu:false,
            pieces_dispo
        }
    }
}
#[derive(PartialEq)]
enum EtatPartie {
    Rejoue,
    Normal,
}

struct InterfaceTetris {
    moteur: Moteur,
    largeur_choisie: i32,
    hauteur_choisie: i32,
    vision_choisie: u16,
    coups_choisis:u16,
    bouton_rotation:button::State,
    bouton_change:button::State,
    bouton_charge:button::State,
    bouton_sauvegarde:button::State,
    entree_fichier:text_input::State,
    slider_largeur:slider::State,
    slider_hauteur:slider::State,
    slider_vision:slider::State,
    slider_coups:slider::State,
    bouton_vider:button::State,
    bouton_rejoue:button::State,
    bouton_info:button::State,
    bouton_auto:button::State,
    texte_fichier:String,
    etat_partie:EtatPartie,
    etape_actuelle:usize,
    pieces_dispo:Vec<(bool, Piece)>,
    mode_jeu:bool,
}

impl InterfaceTetris {
    fn update_pieces_possibles(&mut self, id:usize) {
        
        let mut nb_poss = 0;
        for (est_dispo, piece) in &self.pieces_dispo {
            if *est_dispo {
                nb_poss += 1;
            }
        }
        if nb_poss > 1 && self.pieces_dispo[id].0 {
            self.pieces_dispo[id].0 = false;
        }
        else if nb_poss >= 1 {
            self.pieces_dispo[id].0 = true;
        }
        self.update_pieces_moteur();
        
    }
    fn update_pieces_moteur(&mut self) {
        self.moteur.pieces = Vec::new();
        for (est_dispo, piece) in &self.pieces_dispo {
            if *est_dispo {
                let mut nouvpiece = Piece{orientation:piece.orientation, hauteur:piece.hauteur, positions:Vec::with_capacity(piece.positions.len()), nom:piece.nom.clone()};
                for pos in &piece.positions {
                    nouvpiece.positions.push(*pos);
                }
                self.moteur.pieces.push(nouvpiece);
            }
        }
    }
}

impl Sandbox for InterfaceTetris {
    type Message = Message;

    fn new() -> InterfaceTetris {
        InterfaceTetris::default()
    }

    fn title(&self) -> String {
        String::from("Simulateur de Tetris")
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::Rotation => self.moteur.pieces[self.moteur.piece_choisie].rotation_sens_aiguille(),
            Message::PosePiece(x) if self.etat_partie == EtatPartie::Normal => match self.moteur.jeu.tombe_piece(&self.moteur.pieces[self.moteur.piece_choisie], x) {
                Some(altitude) => {if self.moteur.jeu.rajoute_piece(x as i32, altitude as i32, &self.moteur.pieces[self.moteur.piece_choisie]) { 
                    self.moteur.jeu.iteration_sweep();
                    self.moteur.nouvelle_etape(x, self.moteur.piece_choisie, self.moteur.pieces[self.moteur.piece_choisie].orientation);
                    self.etape_actuelle += 1;
                    if self.mode_jeu {
                        self.moteur.piece_choisie = rand::thread_rng().gen_range(0..self.moteur.pieces.len());
                    }
                }},
                None => (),
            },
            Message::ChangePiece => {
                if self.moteur.piece_choisie == self.moteur.pieces.len() - 1 {
                    self.moteur.piece_choisie = 0;
                }
                else {
                    self.moteur.piece_choisie += 1;
                }
            }
            Message::Sauvegarde => {
                if self.texte_fichier.len() > 2 {
                    self.moteur.partie.sauvegarde_fichier_partie(self.texte_fichier.trim());
                }
            }
            Message::Charge => {
                if self.texte_fichier.len() > 2 {
                    match PartieTetris::charger_depuis_fichier(self.texte_fichier.trim()) {
                        Some(part) => {
                            self.etat_partie = EtatPartie::Rejoue;
                            self.etape_actuelle = 0;
                            self.moteur.vider_tableau();
                            self.largeur_choisie = part.largeur as i32;
                            self.hauteur_choisie = part.hauteur as i32;
                            self.moteur.jeu = Tableau::new(part.hauteur, part.largeur);
                            self.moteur.partie = part;

                        },
                        None => (),
                    }
                    
                }
            }
            Message::Rien => (),
            Message::ChangeTexte(text) => self.texte_fichier = text,
            Message::SliderLargeur(nouv_largeur) => {
                self.largeur_choisie = nouv_largeur;
                let piece = self.moteur.piece_choisie;
                self.moteur = Moteur::new(self.hauteur_choisie as usize, self.largeur_choisie as usize,self.mode_jeu);
                self.moteur.piece_choisie = piece;
                self.moteur.intialisation();
                self.update_pieces_moteur();
                self.moteur.piece_choisie = 0;
                self.moteur.vision.1.clear();
                let mut rand = rand::thread_rng();
                for i in 0..self.moteur.vision.0 {
                    self.moteur.vision.1.push(rand.gen_range(0..self.moteur.pieces.len()) as u16)
                }
                self.etat_partie = EtatPartie::Normal;
                self.etape_actuelle = 0;
            },
            Message::SliderHauteur(nouv_hauteur) => {
                self.hauteur_choisie = nouv_hauteur;
                let piece = self.moteur.piece_choisie;
                self.moteur = Moteur::new(self.hauteur_choisie as usize, self.largeur_choisie as usize,self.mode_jeu);
                self.moteur.piece_choisie = piece;
                self.moteur.intialisation();
                self.update_pieces_moteur();
                self.moteur.piece_choisie = 0;
                self.moteur.vision.1.clear();
                let mut rand = rand::thread_rng();
                for i in 0..self.moteur.vision.0 {
                    self.moteur.vision.1.push(rand.gen_range(0..self.moteur.pieces.len()) as u16)
                }
                self.etat_partie = EtatPartie::Normal;
                self.etape_actuelle = 0;
            },
            Message::SliderVision(nouv_vision) => {
                self.vision_choisie = nouv_vision;
                let piece = self.moteur.piece_choisie;
                self.moteur = Moteur::new(self.hauteur_choisie as usize, self.largeur_choisie as usize,self.mode_jeu);
                self.moteur.piece_choisie = piece;
                self.moteur.vision.0 = nouv_vision;
                self.moteur.intialisation();
                self.update_pieces_moteur();
                self.moteur.piece_choisie = 0;
                self.moteur.vision.1.clear();
                let mut rand = rand::thread_rng();
                for i in 0..self.moteur.vision.0 {
                    self.moteur.vision.1.push(rand.gen_range(0..self.moteur.pieces.len()) as u16)
                }
                self.etat_partie = EtatPartie::Normal;
                self.etape_actuelle = 0;
            },
            Message::ViderTableau => {
                self.moteur.jeu.vider();
                self.moteur.partie = PartieTetris::new(self.moteur.jeu.hauteur, self.moteur.jeu.largeur);
                self.etat_partie = EtatPartie::Normal;
                self.etape_actuelle = 0;
            },
            Message::Rejouer => {
                match self.etat_partie {
                    EtatPartie::Normal => {if self.moteur.partie.deroulement.len() > 0 {self.etat_partie = EtatPartie::Rejoue; self.etape_actuelle = 0; self.moteur.vider_tableau();}},
                    EtatPartie::Rejoue => {if self.etape_actuelle == self.moteur.partie.deroulement.len() - 1 {
                        self.moteur.joue_etape(self.etape_actuelle);
                        self.etat_partie = EtatPartie::Normal;
                        self.etape_actuelle += 1;
                    }
                    else {
                        self.moteur.joue_etape(self.etape_actuelle);
                        self.etape_actuelle += 1;
                    }
                },
                }
            },
            Message::ChangeMode(nv_mode) => {
                self.mode_jeu = nv_mode;
                if self.mode_jeu {
                    self.hauteur_choisie = 25;
                    self.largeur_choisie = 10;
                }
                self.update(Message::SliderLargeur(self.largeur_choisie));
                self.update(Message::SliderHauteur(self.hauteur_choisie));
            },
            Message::FaitInfographique => {
                self.moteur.cree_infograph();
            },
            Message::JoueAuto => {
                for i in 0..self.coups_choisis {
                    self.moteur.joue_algo_trouve(&self.moteur.vision.2.clone());
                    self.etape_actuelle += 1;
                }
            },
            Message::SliderCoups(nv_val) => {
                self.coups_choisis = nv_val;
            },
            Message::ChangePossPiece(val,id) => {
                self.update_pieces_possibles(id as usize);
                self.moteur.jeu.vider();
                self.moteur.partie = PartieTetris::new(self.moteur.jeu.hauteur, self.moteur.jeu.largeur);
                self.etat_partie = EtatPartie::Normal;
                self.etape_actuelle = 0;
                self.moteur.piece_choisie = 0;
                self.moteur.vision.1.clear();
                let mut rand = rand::thread_rng();
                for i in 0..self.moteur.vision.0 {
                    self.moteur.vision.1.push(rand.gen_range(0..self.moteur.pieces.len()) as u16)
                }
            }
            _ => (),
        }
    }

    fn view(&mut self) -> Element<Self::Message> {
        let mut pieces_poss = Column::new().padding(20)
        .spacing(20)
        .align_items(Align::Start);
        for i in 0..self.pieces_dispo.len() {
            let (est_dispo, piece) = &mut self.pieces_dispo[i];
            let ident = i as u32;
            pieces_poss = pieces_poss.push(
                Row::new().align_items(Align::Start)
                .push(Checkbox::new(*est_dispo, format!("{}", piece.nom.clone()), move |x| {Message::ChangePossPiece(x,ident)}))
            );
        }
        pieces_poss = pieces_poss.push(Text::new("Vision :"));
        for i in 0..self.moteur.vision.0 {
            pieces_poss = pieces_poss.push(Text::new(format!("{} - {}", i+1,self.moteur.pieces[self.moteur.vision.1[i as usize] as usize].nom).trim()))
        }

        if self.mode_jeu {
            Row::new().align_items(Align::Start).push(
                Column::new()
                    .padding(20)
                    .spacing(20)
                    .align_items(Align::Start)
                    .push(
                Button::new(&mut self.bouton_rotation, Text::new("Faire tourner la piece"))
                            .padding(8)
                            .on_press(Message::Rotation)
                    ).push(
                        Button::new(&mut self.bouton_change, Text::new("Changer de piece"))
                            .padding(8)
                            .on_press(Message::Rien)
                    )
                    .push(
                    Row::new().align_items(Align::Start).push(
                        Button::new(&mut self.bouton_charge, Text::new("Charger une partie"))
                            .padding(8)
                            .on_press(Message::Charge)
                        ).push(
                            Button::new(&mut self.bouton_sauvegarde, Text::new("Sauvegarder une partie"))
                            .padding(8)
                            .on_press(Message::Sauvegarde)
                        )
                    ).push(
                        TextInput::new(&mut self.entree_fichier, "Nom de fichier ici", &mut self.texte_fichier, |x|{Message::ChangeTexte(x)}).padding(10).width(Length::Units(300))
                    ).push(
                        Row::new().align_items(Align::Start).push(
                            Text::new(format!("Largeur : {}", self.largeur_choisie))
                        ).push(
                            slider::Slider::new(&mut self.slider_largeur, std::ops::RangeInclusive::new(1,30), self.largeur_choisie, |x| {Message::Rien}).width(Length::Units(300))
                        )
                    )
                    .push(
                        Row::new().align_items(Align::Start).push(
                            Text::new(format!("Hauteur : {}", self.hauteur_choisie))
                        ).push(
                        slider::Slider::new(&mut self.slider_hauteur, std::ops::RangeInclusive::new(1,30), self.hauteur_choisie, |x| {Message::Rien}).width(Length::Units(300))
                        )
                    ).push(
                        Row::new().align_items(Align::Center).push(
                            Text::new(format!("Etape {}/{}", self.etape_actuelle, self.moteur.partie.deroulement.len()))
                        ).push(
                            Button::new(&mut self.bouton_rejoue, Text::new("Rejouer")).on_press(Message::Rejouer)
                        ).push(
                            Button::new(&mut self.bouton_info, Text::new("Infographique")).on_press(Message::FaitInfographique)
                        )
                    )
                    .push(
                        Row::new().align_items(Align::Center).push(
                            Checkbox::new(self.mode_jeu, "Activer le mode jeu",|x|{Message::ChangeMode(x)} )
                        )
                    )
                ).push(
                    Column::new().align_items(Align::Center).push(
                        Text::new("Tableau de jeu")
                    ).push(
                        self.moteur.view()
                    )
                    .push(
                        Button::new(&mut self.bouton_vider, Text::new("Vider le tableau")).padding(8).on_press(Message::ViderTableau)
                    )
                ).into()
        }
        else {
        Row::new().align_items(Align::Start).push(
        Column::new()
            .padding(20)
            .spacing(20)
            .align_items(Align::Start)
            .push(
        Button::new(&mut self.bouton_rotation, Text::new("Faire tourner la piece"))
                    .padding(8)
                    .on_press(Message::Rotation)
            ).push(
                Button::new(&mut self.bouton_change, Text::new("Changer de piece"))
                    .padding(8)
                    .on_press(Message::ChangePiece)
            )
            .push(
            Row::new().align_items(Align::Start).push(
                Button::new(&mut self.bouton_charge, Text::new("Charger une partie"))
                    .padding(8)
                    .on_press(Message::Charge)
                ).push(
                    Button::new(&mut self.bouton_sauvegarde, Text::new("Sauvegarder une partie"))
                    .padding(8)
                    .on_press(Message::Sauvegarde)
                )
            ).push(
                TextInput::new(&mut self.entree_fichier, "Nom de fichier ici", &mut self.texte_fichier, |x|{Message::ChangeTexte(x)}).padding(10).width(Length::Units(300))
            ).push(
                Row::new().align_items(Align::Start).push(
                    Text::new(format!("Largeur : {}", self.largeur_choisie))
                ).push(
                    slider::Slider::new(&mut self.slider_largeur, std::ops::RangeInclusive::new(1,30), self.largeur_choisie, |x| {Message::SliderLargeur(x)}).width(Length::Units(300))
                )
            )
            .push(
                Row::new().align_items(Align::Start).push(
                    Text::new(format!("Hauteur : {}", self.hauteur_choisie))
                ).push(
                slider::Slider::new(&mut self.slider_hauteur, std::ops::RangeInclusive::new(1,30), self.hauteur_choisie, |x| {Message::SliderHauteur(x)}).width(Length::Units(300))
                )
            ).push(
                Row::new().align_items(Align::Center).push(
                    Button::new(&mut self.bouton_auto, Text::new(format!("Joue auto | vision {}", self.vision_choisie))).on_press(Message::JoueAuto)
                    ).push(
                        slider::Slider::new(&mut self.slider_vision, std::ops::RangeInclusive::new(1,5), self.vision_choisie, |x| {Message::SliderVision(x)}).width(Length::Units(200))
                    )
            ).push(
                Row::new().align_items(Align::Center).push(
                    Text::new(format!("coups à jouer: {}", self.coups_choisis))
                    ).push(
                        slider::Slider::new(&mut self.slider_coups, std::ops::RangeInclusive::new(1,100), self.coups_choisis, |x| {Message::SliderCoups(x)}).width(Length::Units(200))
                    )
            ).push(
                Row::new().align_items(Align::Center).push(
                    Text::new(format!("Etape {}/{}", self.etape_actuelle, self.moteur.partie.deroulement.len()))
                ).push(
                    Button::new(&mut self.bouton_rejoue, Text::new("Rejouer")).on_press(Message::Rejouer)
                )
                .push(
                    Button::new(&mut self.bouton_info, Text::new("Infographique")).on_press(Message::FaitInfographique)
                )
            )
            .push(
                Row::new().align_items(Align::Center).push(
                    Checkbox::new(self.mode_jeu, "Activer le mode jeu",|x|{Message::ChangeMode(x)} )
                )
            )
        ).push(
            Column::new().align_items(Align::Center).push(
                Text::new("Tableau de jeu")
            ).push(
                self.moteur.view()
            )
            .push(
                Button::new(&mut self.bouton_vider, Text::new("Vider le tableau")).padding(8).on_press(Message::ViderTableau)
            )
        ).push(pieces_poss).into()
        }
    }
}

impl Program<Message> for Piece {
    fn draw(&self, bounds:Rectangle, _cursor:Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(bounds.size());
        for case in &self.positions {
            let rect = Path::rectangle(Point::new((case.x as f32) * TAILLE_CASE as f32, (case.y as f32) * TAILLE_CASE as f32), Size::new(TAILLE_CASE as f32, TAILLE_CASE as f32));
            frame.fill(&rect, Color::from_rgb(0.5, 0.0, 0.0));
        }

        vec![frame.into_geometry()]
    }
    fn update(&mut self, event: Event, bounds: Rectangle, cursor: Cursor,) -> (event::Status, Option<Message>) {
        (event::Status::Ignored, None)
    }
    fn mouse_interaction(
        &self,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> mouse::Interaction {
        mouse::Interaction::default()
    }
}

impl Piece {
    fn view<'a>(&'a mut self) -> Element<'a, Message> {
        let hauteur = (self.positions.len() * TAILLE_CASE) as u16;
        let largeur = (self.positions.len() * TAILLE_CASE) as u16;
        Canvas::new(self)
            .width(Length::Units(largeur))
            .height(Length::Units(hauteur))
            .into()
    }
}

impl Program<Message> for Moteur {
    fn draw(&self, bounds:Rectangle, _cursor:Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(bounds.size());
        for x in 0..self.jeu.largeur {
            for y in 0..self.jeu.hauteur {
                let rect = Path::rectangle(Point::new((x * TAILLE_CASE) as f32, ((self.jeu.hauteur - 1 - y) * TAILLE_CASE) as f32), Size::new(TAILLE_CASE as f32, TAILLE_CASE as f32));
                if self.jeu.rempli_a(x as i32, y as i32) {
                    frame.fill(&rect, Color::from_rgb(0.0, 1.0, 0.0));
                }
                else {
                    frame.fill(&rect, Color::BLACK);
                }
            }
        }
        match self.interaction {
            Interaction::DansTableau(x, y) => {
                let piece = &self.pieces[self.piece_choisie];
                if self.jeu.tient_dans(x as i32, self.jeu.hauteur as i32 - y as i32 - 1, piece) {
                    for case in &piece.positions {
                        let rect = Path::rectangle(Point::new((x + case.x as f32) * TAILLE_CASE as f32, (y - case.y as f32) * TAILLE_CASE as f32), Size::new(TAILLE_CASE as f32, TAILLE_CASE as f32));
                        frame.fill(&rect, Color::from_rgb(1.0, 0.0, 0.0));
                    }
                }
            }
            Interaction::Rien => (),
        }


        vec![frame.into_geometry()]
    }
    fn update(&mut self, event: Event, bounds: Rectangle, cursor: Cursor,) -> (event::Status, Option<Message>) {
        let cursor_position =
                if let Some(position) = cursor.position_in(&bounds) {
                    self.interaction = Interaction::DansTableau((position.x/TAILLE_CASE as f32).floor(), (position.y/TAILLE_CASE as f32).floor());
                    position
                } else {
                    return (event::Status::Ignored, None);
                };
        match event {
            Event::Mouse(mouse_event) => match mouse_event {
                mouse::Event::ButtonPressed(button) => {
                    let message = match button {
                        mouse::Button::Left => {
                            Some(Message::PosePiece((cursor_position.x/TAILLE_CASE as f32).floor() as usize))
                        }
                        mouse::Button::Right => {
                            Some(Message::Rotation)
                        }
                        _ => None,
                    };

                    return (event::Status::Captured, message)
                },
                mouse::Event::WheelScrolled { delta } if !self.mode_jeu => match delta {
                    mouse::ScrollDelta::Lines { y, .. }
                    | mouse::ScrollDelta::Pixels { y, .. } => {
                        if y > 0.0 {
                            self.piece_choisie += 1;
                            if self.piece_choisie >= self.pieces.len() {self.piece_choisie = 0}
                        }
                        else if y < 0.0 {
                            if self.piece_choisie == 0 {self.piece_choisie = self.pieces.len() - 1}
                            else {self.piece_choisie -= 1;}
                        }
                    }
                }
                _ => (),
            },
            _ => (),
        }
        
        
        (event::Status::Ignored, None)
    }
    fn mouse_interaction(
        &self,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> mouse::Interaction {
        mouse::Interaction::default()
    }
}

impl Moteur {
    fn view<'a>(&'a mut self) -> Element<'a, Message> {
        let hauteur = (self.jeu.hauteur * TAILLE_CASE) as u16;
        let largeur = (self.jeu.largeur * TAILLE_CASE) as u16;
        Canvas::new(self)
            .width(Length::Units(largeur))
            .height(Length::Units(hauteur))
            .into()
    }
}

#[derive(Clone, Copy)]
struct ColRGB {
    r:u8,
    g:u8,
    b:u8
}

impl ColRGB {
    fn new(r:u8,g:u8,b:u8) -> ColRGB {
        ColRGB{r,g,b}
    }
}

struct Framebuffer {
    data:Vec<ColRGB>,
    long:usize,
    larg:usize,
}

impl Framebuffer {
    fn new(long:usize, larg:usize, colinit:ColRGB) -> Framebuffer {
        Framebuffer {long, larg, data:vec![colinit; long*larg]}
    }
    fn mod_a(&mut self, (x,y):(usize,usize), nouv:ColRGB) {
        self.data[y * self.long + x] = nouv;
    }
    fn ligne(&mut self, (xs,ys):(f32,f32), (xf,yf):(f32,f32), cols:ColRGB, colf:ColRGB) {
        let dist = ((xf - xs).powf(2.0) + (yf - ys).powf(2.0)).sqrt();
        let ang = (yf - ys).atan2(xf - xs);
        let (mut x,mut y) = (xs, ys);
        let mut coef = 1.0;
        for i in 0..dist as usize + 1 {
            coef = (dist as usize - i) as f32 / dist; 
            self.mod_a((x as usize, y as usize), ColRGB::new((cols.r as f32 * coef + colf.r as f32 * (1.0 - coef)) as u8, (cols.g as f32 * coef + colf.g as f32 * (1.0 - coef)) as u8, (cols.b as f32 * coef + colf.b as f32 * (1.0 - coef)) as u8));
            x += ang.cos();
            y += ang.sin();
        }
    }

    fn rectangle(&mut self, (xs,ys):(usize, usize), (xf,yf):(usize,usize), col:ColRGB) {
        for x in xs..xf + 1 {
            for y in ys..yf + 1 {
                self.mod_a((x, y),col);
            }
        }
    }
    fn pointe_horiz(&mut self, (xs,ys):(f32,f32), (xf,yf):(f32,f32),larg:i32, cols:ColRGB, colf:ColRGB) {
        for alt in -larg..larg {
            self.ligne((xs, ys + alt as f32), (xf,yf), cols, colf);
        }
    }
}

fn menu_simulation() {
    let (tunnel_send, tunnel_rcv):(Sender<(CoefAlgo, u32)>, Receiver<(CoefAlgo, u32)>) = mpsc::channel();
    let vision = demande_input_recurs::<u16>("Vision voulue ? (de 1 à 5)").clamp(1,5);
    let nombre_threads = demande_input_recurs::<u32>("Nombre de threads ? (de 1 à beaucoup)").clamp(1,4096);
    let nombre_simulation = demande_input_recurs::<u32>("Nombre de simulation totales ? (du nombre de threads à 1 000 000 000)").clamp(nombre_threads,1000000000);
    let maxcoups = demande_input_recurs::<u32>("Nombre de coups maximum par simulation ? (de 100 à 10 000 000)").clamp(100,10000000);
    let nom_result = demande_input("Nom du fichier de résultats ?");
    for i in 0..nombre_threads {
        let tunnel_send_c = tunnel_send.clone();
        thread::spawn(move || {
            let mut moteur = Moteur::new(20,10,false);
            moteur.intialisation();
            println!("thread {} lancé", i);
            tunnel_send_c.send(moteur.simulation_vision_n_coef_solo(vision, CoefAlgo { rempdess: 0, rempsides: 0, alt: 0, nb_sweep: 0, tr_imp_c: 0 }, CoefAlgo { rempdess: 50, rempsides: 50, alt: 50, nb_sweep: 50, tr_imp_c: 50 },maxcoups, nombre_simulation/nombre_threads,i));
        });
    }
    let mut meilleurs_coefs = (CoefAlgo { rempdess: 0, rempsides: 0, alt: 0, nb_sweep: 0, tr_imp_c: 0 }, 0);
    for i in 0..nombre_threads {
        let (coefs, score) = tunnel_rcv.recv().unwrap();
        if score > meilleurs_coefs.1 {
            meilleurs_coefs = (coefs, score)
        }
    }
    let (coefs, score) = meilleurs_coefs;
    ecrit_resultats(nom_result.trim(), vec![(coefs,score)]);
    println!("meilleurs coefs : {} {} {} {} {}, meilleur score:{}",coefs.rempdess,coefs.rempsides,coefs.alt,coefs.nb_sweep,coefs.tr_imp_c, score);

}

fn ecrit_resultats(nom:&str, data:Vec<(CoefAlgo, u32)>) {
    create_dir("Résultats");
    match File::create(format!("Résultats/{}.txt", nom)) {
        Ok(fic) => {
            let mut scripteur = fic;
            let mut data_fichier = String::new();
            data_fichier.push_str("rempdess,repsides,alt,nb_sweep,tr_imp_c,score\n");
            for (coefs, score) in &data {
                data_fichier.push_str(format!("{},{},{},{},{},{}\n",coefs.rempdess,coefs.rempsides,coefs.alt,coefs.nb_sweep,coefs.tr_imp_c, score).trim_matches('\t'));
            }
            write!(&mut scripteur, "{}", data_fichier.trim_matches('\t')).unwrap();
            println!("fichier de résultats sauvegardé !")
        },
        Err(_err) => println!("FQFQSF"),
            
    }
}

fn demande_input(texte_input: &str) -> String {
    // similaire à input() de python
    // prend un prompt en entrée et sort un String
    let mut input = String::new();
    println!("{}", texte_input);
    match io::stdin().read_line(&mut input) {
        Ok(_) => {
            ()
        },
        Err(e) => println!("pas swag : {}", e)
    }
    input
}

fn demande_input_recurs<T:FromStr>(text_input:&str) -> T {
    match T::from_str(&demande_input(text_input).trim()) {
        Ok(output) => return output,
        Err(_) => return demande_input_recurs(text_input),
    }
}

//fn main() {
//    menu_simulation();
//}

// best sur 1 = 27 21 4 2 6 score:36599
// best sur 2 = 35 24 24 30 14 score:100000 (limite)
fn main() -> iced::Result {
    let settings = Settings {
        window: window::Settings {
            size: (600 + TAILLE_CASE as u32 * 30,if TAILLE_CASE * 30 > 430 { TAILLE_CASE as u32 * 30 + 60} else {430}),
            resizable: true,
            decorations: true,
            ..Default::default()
        },
        ..Default::default()
    };
    InterfaceTetris::run(settings)
}
