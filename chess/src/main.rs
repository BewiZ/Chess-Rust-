use rand::Rng;
use std::env;
use std::fmt;
use std::io;
use tokio;

// 导入自定义模块
mod api_client;
mod fen_converter;
use crate::api_client::SiliconFlowClient;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn opposite(&self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Color::White => write!(f, "白方"),
            Color::Black => write!(f, "黑方"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Piece {
    King(Color, bool),
    Queen(Color),
    Rook(Color, bool),
    Bishop(Color),
    Knight(Color),
    Pawn(Color, bool),
}

impl Piece {
    pub fn color(&self) -> Color {
        match self {
            Piece::King(color, _) => *color,
            Piece::Queen(color) => *color,
            Piece::Rook(color, _) => *color,
            Piece::Bishop(color) => *color,
            Piece::Knight(color) => *color,
            Piece::Pawn(color, _) => *color,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Piece::King(_, _) => "王",
            Piece::Queen(_) => "后",
            Piece::Rook(_, _) => "车",
            Piece::Bishop(_) => "象",
            Piece::Knight(_) => "马",
            Piece::Pawn(_, _) => "兵",
        }
    }
}

pub type Square = Option<Piece>;

#[derive(Debug, Clone)]
pub struct Chessboard {
    board: [[Square; 8]; 8],
    current_turn: Color,
    castling_rights: CastlingRights,
    en_passant_target: Option<Position>,
    move_history: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CastlingRights {
    pub white_kingside: bool,
    pub white_queenside: bool,
    pub black_kingside: bool,
    pub black_queenside: bool,
}

impl CastlingRights {
    pub fn new() -> Self {
        Self {
            white_kingside: true,
            white_queenside: true,
            black_kingside: true,
            black_queenside: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub row: usize,
    pub col: usize,
}

impl Position {
    pub fn new(row: usize, col: usize) -> Option<Self> {
        if row < 8 && col < 8 {
            Some(Self { row, col })
        } else {
            None
        }
    }

    pub fn from_notation(notation: &str) -> Option<Self> {
        if notation.len() != 2 {
            return None;
        }
        let mut chars = notation.chars();
        let col_char = chars.next()?;
        let row_char = chars.next()?;

        let col = match col_char {
            'a'..='h' => (col_char as usize) - ('a' as usize),
            _ => return None,
        };

        let row = match row_char {
            '1'..='8' => 8 - (row_char as usize - '1' as usize) - 1,
            _ => return None,
        };

        Some(Self { row, col })
    }

    pub fn to_notation(&self) -> String {
        format!("{}{}", (b'a' + self.col as u8) as char, 8 - self.row)
    }
}

#[derive(Debug, Clone)]
pub struct Move {
    pub from: Position,
    pub to: Position,
    pub promotion: Option<Piece>,
}

impl Move {
    pub fn from_notation(notation: &str) -> Option<Self> {
        let parts: Vec<&str> = notation.split_whitespace().collect();
        if parts.len() < 2 {
            return None;
        }

        let from = Position::from_notation(parts[0])?;
        let to = Position::from_notation(parts[1])?;

        Some(Move {
            from,
            to,
            promotion: None,
        })
    }

    pub fn to_notation(&self) -> String {
        format!("{} {}", self.from.to_notation(), self.to.to_notation())
    }
}

impl Chessboard {
    pub fn new() -> Self {
        let mut board = [[None; 8]; 8];

        // 初始化兵
        for col in 0..8 {
            board[1][col] = Some(Piece::Pawn(Color::Black, false));
            board[6][col] = Some(Piece::Pawn(Color::White, false));
        }

        // 初始化其他棋子 - 黑方
        board[0][0] = Some(Piece::Rook(Color::Black, false));
        board[0][1] = Some(Piece::Knight(Color::Black));
        board[0][2] = Some(Piece::Bishop(Color::Black));
        board[0][3] = Some(Piece::Queen(Color::Black));
        board[0][4] = Some(Piece::King(Color::Black, false));
        board[0][5] = Some(Piece::Bishop(Color::Black));
        board[0][6] = Some(Piece::Knight(Color::Black));
        board[0][7] = Some(Piece::Rook(Color::Black, false));

        // 初始化其他棋子 - 白方
        board[7][0] = Some(Piece::Rook(Color::White, false));
        board[7][1] = Some(Piece::Knight(Color::White));
        board[7][2] = Some(Piece::Bishop(Color::White));
        board[7][3] = Some(Piece::Queen(Color::White));
        board[7][4] = Some(Piece::King(Color::White, false));
        board[7][5] = Some(Piece::Bishop(Color::White));
        board[7][6] = Some(Piece::Knight(Color::White));
        board[7][7] = Some(Piece::Rook(Color::White, false));

        Chessboard {
            board,
            current_turn: Color::White,
            castling_rights: CastlingRights::new(),
            en_passant_target: None,
            move_history: Vec::new(),
        }
    }

    pub fn get(&self, pos: Position) -> Square {
        self.board[pos.row][pos.col]
    }

    pub fn current_turn(&self) -> Color {
        self.current_turn
    }

    // 获取所有合法移动
    pub fn get_legal_moves(&self, from: Position) -> Vec<Move> {
        let mut moves = Vec::new();

        let piece = match self.get(from) {
            Some(piece) => piece,
            None => return moves,
        };

        if piece.color() != self.current_turn {
            return moves;
        }

        match piece {
            Piece::Pawn(color, _) => self.pawn_moves(from, color, &mut moves),
            Piece::Knight(color) => self.knight_moves(from, color, &mut moves),
            Piece::Bishop(color) => self.bishop_moves(from, color, &mut moves),
            Piece::Rook(color, _) => self.rook_moves(from, color, &mut moves),
            Piece::Queen(color) => self.queen_moves(from, color, &mut moves),
            Piece::King(color, _) => self.king_moves(from, color, &mut moves),
        }

        // 过滤掉会导致自己被将军的移动
        moves
            .into_iter()
            .filter(|mv| {
                let mut test_board = self.clone();
                test_board.make_move_unchecked(mv);
                !test_board.is_in_check(piece.color())
            })
            .collect()
    }

    // 随机合法走法（新增方法）
    pub fn get_random_legal_move(&self) -> Option<Move> {
        let mut all_legal_moves = Vec::new();

        // 收集所有合法走法
        for row in 0..8 {
            for col in 0..8 {
                let pos = Position::new(row, col).unwrap();
                let moves = self.get_legal_moves(pos);
                all_legal_moves.extend(moves);
            }
        }

        if all_legal_moves.is_empty() {
            return None;
        }

        // 随机选择一个走法
        let mut rng = rand::thread_rng();
        let random_index = rng.gen_range(0..all_legal_moves.len());
        Some(all_legal_moves[random_index].clone())
    }

    // 兵的移动逻辑
    fn pawn_moves(&self, from: Position, color: Color, moves: &mut Vec<Move>) {
        let direction = match color {
            Color::White => -1,
            Color::Black => 1,
        };

        let new_row = from.row as i32 + direction;
        if new_row < 0 || new_row >= 8 {
            return;
        }

        let new_row = new_row as usize;

        // 前进一格
        if self.board[new_row][from.col].is_none() {
            self.add_pawn_move(from, new_row, from.col, color, moves);

            // 前进两格（初始位置）
            let start_row = match color {
                Color::White => 6,
                Color::Black => 1,
            };
            if from.row == start_row {
                let double_row = (from.row as i32 + 2 * direction) as usize;
                if self.board[double_row][from.col].is_none() {
                    moves.push(Move {
                        from,
                        to: Position {
                            row: double_row,
                            col: from.col,
                        },
                        promotion: None,
                    });
                }
            }
        }

        // 吃子（左侧）
        if from.col > 0 {
            let left_col = from.col - 1;
            if self.can_capture(Position::new(new_row, left_col).unwrap(), color) {
                self.add_pawn_move(from, new_row, left_col, color, moves);
            }
        }

        // 吃子（右侧）
        if from.col < 7 {
            let right_col = from.col + 1;
            if self.can_capture(Position::new(new_row, right_col).unwrap(), color) {
                self.add_pawn_move(from, new_row, right_col, color, moves);
            }
        }

        // 吃过路兵
        if let Some(en_passant_pos) = self.en_passant_target {
            if en_passant_pos.row == new_row
                && (en_passant_pos.col as i32 - from.col as i32).abs() == 1
            {
                let en_passant_direction = match color {
                    Color::White => -1,
                    Color::Black => 1,
                };
                let pawn_behind_row = (en_passant_pos.row as i32 - en_passant_direction) as usize;

                if let Some(Piece::Pawn(opponent_color, _)) =
                    self.board[pawn_behind_row][en_passant_pos.col]
                {
                    if opponent_color != color {
                        moves.push(Move {
                            from,
                            to: en_passant_pos,
                            promotion: None,
                        });
                    }
                }
            }
        }
    }

    fn add_pawn_move(
        &self,
        from: Position,
        to_row: usize,
        to_col: usize,
        color: Color,
        moves: &mut Vec<Move>,
    ) {
        let promotion_row = match color {
            Color::White => 0,
            Color::Black => 7,
        };

        if to_row == promotion_row {
            // 升变选择
            let promotions = [
                Piece::Queen(color),
                Piece::Rook(color, true),
                Piece::Bishop(color),
                Piece::Knight(color),
            ];
            for &promotion in &promotions {
                moves.push(Move {
                    from,
                    to: Position {
                        row: to_row,
                        col: to_col,
                    },
                    promotion: Some(promotion),
                });
            }
        } else {
            moves.push(Move {
                from,
                to: Position {
                    row: to_row,
                    col: to_col,
                },
                promotion: None,
            });
        }
    }

    // 马的移动逻辑
    fn knight_moves(&self, from: Position, color: Color, moves: &mut Vec<Move>) {
        let knight_moves = [
            (-2, -1),
            (-2, 1),
            (-1, -2),
            (-1, 2),
            (1, -2),
            (1, 2),
            (2, -1),
            (2, 1),
        ];

        for &(dr, dc) in &knight_moves {
            let new_row = from.row as i32 + dr;
            let new_col = from.col as i32 + dc;

            if new_row >= 0 && new_row < 8 && new_col >= 0 && new_col < 8 {
                let new_row = new_row as usize;
                let new_col = new_col as usize;
                let to_pos = Position::new(new_row, new_col).unwrap();

                if self.can_move_to(to_pos, color) {
                    moves.push(Move {
                        from,
                        to: to_pos,
                        promotion: None,
                    });
                }
            }
        }
    }

    // 象的移动逻辑
    fn bishop_moves(&self, from: Position, color: Color, moves: &mut Vec<Move>) {
        let directions = [(-1, -1), (-1, 1), (1, -1), (1, 1)];
        self.sliding_moves(from, color, &directions, moves);
    }

    // 车的移动逻辑
    fn rook_moves(&self, from: Position, color: Color, moves: &mut Vec<Move>) {
        let directions = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        self.sliding_moves(from, color, &directions, moves);
    }

    // 后的移动逻辑
    fn queen_moves(&self, from: Position, color: Color, moves: &mut Vec<Move>) {
        let directions = [
            (-1, -1),
            (-1, 1),
            (1, -1),
            (1, 1),
            (-1, 0),
            (1, 0),
            (0, -1),
            (0, 1),
        ];
        self.sliding_moves(from, color, &directions, moves);
    }

    // 王的移动逻辑（包括王车易位）
    fn king_moves(&self, from: Position, color: Color, moves: &mut Vec<Move>) {
        let king_moves = [
            (-1, -1),
            (-1, 0),
            (-1, 1),
            (0, -1),
            (0, 1),
            (1, -1),
            (1, 0),
            (1, 1),
        ];

        for &(dr, dc) in &king_moves {
            let new_row = from.row as i32 + dr;
            let new_col = from.col as i32 + dc;

            if new_row >= 0 && new_row < 8 && new_col >= 0 && new_col < 8 {
                let new_row = new_row as usize;
                let new_col = new_col as usize;
                let to_pos = Position::new(new_row, new_col).unwrap();

                if self.can_move_to(to_pos, color) {
                    moves.push(Move {
                        from,
                        to: to_pos,
                        promotion: None,
                    });
                }
            }
        }

        // 王车易位
        self.castling_moves(from, color, moves);
    }

    // 王车易位逻辑
    fn castling_moves(&self, from: Position, color: Color, moves: &mut Vec<Move>) {
        if self.is_in_check(color) {
            return;
        }

        let (kingside_right, queenside_right, back_rank) = match color {
            Color::White => (
                self.castling_rights.white_kingside,
                self.castling_rights.white_queenside,
                7,
            ),
            Color::Black => (
                self.castling_rights.black_kingside,
                self.castling_rights.black_queenside,
                0,
            ),
        };

        // 短易位（王翼易位）
        if kingside_right {
            if self.board[back_rank][5].is_none()
                && self.board[back_rank][6].is_none()
                && !self.is_square_attacked(Position::new(back_rank, 4).unwrap(), color.opposite())
                && !self.is_square_attacked(Position::new(back_rank, 5).unwrap(), color.opposite())
                && !self.is_square_attacked(Position::new(back_rank, 6).unwrap(), color.opposite())
            {
                moves.push(Move {
                    from,
                    to: Position {
                        row: back_rank,
                        col: 6,
                    },
                    promotion: None,
                });
            }
        }

        // 长易位（后翼易位）
        if queenside_right {
            if self.board[back_rank][1].is_none()
                && self.board[back_rank][2].is_none()
                && self.board[back_rank][3].is_none()
                && !self.is_square_attacked(Position::new(back_rank, 2).unwrap(), color.opposite())
                && !self.is_square_attacked(Position::new(back_rank, 3).unwrap(), color.opposite())
                && !self.is_square_attacked(Position::new(back_rank, 4).unwrap(), color.opposite())
            {
                moves.push(Move {
                    from,
                    to: Position {
                        row: back_rank,
                        col: 2,
                    },
                    promotion: None,
                });
            }
        }
    }

    // 滑动棋子（象、车、后）的通用移动逻辑
    fn sliding_moves(
        &self,
        from: Position,
        color: Color,
        directions: &[(i32, i32)],
        moves: &mut Vec<Move>,
    ) {
        for &(dr, dc) in directions {
            let mut new_row = from.row as i32 + dr;
            let mut new_col = from.col as i32 + dc;

            while new_row >= 0 && new_row < 8 && new_col >= 0 && new_col < 8 {
                let new_row_usize = new_row as usize;
                let new_col_usize = new_col as usize;
                let to_pos = Position::new(new_row_usize, new_col_usize).unwrap();

                if self.board[new_row_usize][new_col_usize].is_none() {
                    moves.push(Move {
                        from,
                        to: to_pos,
                        promotion: None,
                    });
                } else {
                    if self.can_capture(to_pos, color) {
                        moves.push(Move {
                            from,
                            to: to_pos,
                            promotion: None,
                        });
                    }
                    break;
                }

                new_row += dr;
                new_col += dc;
            }
        }
    }

    fn can_move_to(&self, to: Position, color: Color) -> bool {
        match self.board[to.row][to.col] {
            Some(piece) => piece.color() != color,
            None => true,
        }
    }

    fn can_capture(&self, to: Position, color: Color) -> bool {
        match self.board[to.row][to.col] {
            Some(piece) => piece.color() != color,
            None => false,
        }
    }

    pub fn make_move(&mut self, mv: &Move) -> Result<(), String> {
        let legal_moves = self.get_legal_moves(mv.from);
        if !legal_moves
            .iter()
            .any(|legal_move| legal_move.from == mv.from && legal_move.to == mv.to)
        {
            return Err("非法的移动".to_string());
        }

        let move_notation = mv.to_notation();
        if let Some(promotion) = mv.promotion {
            let promotion_symbol = match promotion {
                Piece::Queen(_) => "Q",
                Piece::Rook(_, _) => "R",
                Piece::Bishop(_) => "B",
                Piece::Knight(_) => "N",
                _ => "",
            };
            self.move_history
                .push(format!("{}{}", move_notation, promotion_symbol));
        } else {
            self.move_history.push(move_notation);
        }

        self.make_move_unchecked(mv);
        Ok(())
    }

    fn make_move_unchecked(&mut self, mv: &Move) {
        let piece = self.board[mv.from.row][mv.from.col].take().unwrap();

        // 处理王车易位
        if let Piece::King(color, _) = piece {
            if (mv.from.col as i32 - mv.to.col as i32).abs() == 2 {
                if mv.to.col == 6 {
                    let rook = self.board[mv.from.row][7].take().unwrap();
                    self.board[mv.from.row][5] = Some(rook);
                } else if mv.to.col == 2 {
                    let rook = self.board[mv.from.row][0].take().unwrap();
                    self.board[mv.from.row][3] = Some(rook);
                }
            }

            match color {
                Color::White => {
                    self.castling_rights.white_kingside = false;
                    self.castling_rights.white_queenside = false;
                }
                Color::Black => {
                    self.castling_rights.black_kingside = false;
                    self.castling_rights.black_queenside = false;
                }
            }
        }

        // 处理车移动（更新易位权利）
        if let Piece::Rook(color, _) = piece {
            match color {
                Color::White => {
                    if mv.from.col == 0 {
                        self.castling_rights.white_queenside = false;
                    } else if mv.from.col == 7 {
                        self.castling_rights.white_kingside = false;
                    }
                }
                Color::Black => {
                    if mv.from.col == 0 {
                        self.castling_rights.black_queenside = false;
                    } else if mv.from.col == 7 {
                        self.castling_rights.black_kingside = false;
                    }
                }
            }
        }

        // 处理兵的移动
        let mut is_en_passant = false;
        if let Piece::Pawn(_color, _) = piece {
            if let Some(en_passant_pos) = self.en_passant_target {
                if mv.to.row == en_passant_pos.row && mv.to.col == en_passant_pos.col {
                    is_en_passant = true;
                    let capture_row = mv.from.row;
                    self.board[capture_row][mv.to.col] = None;
                }
            }

            if (mv.from.row as i32 - mv.to.row as i32).abs() == 2 {
                let en_passant_row = (mv.from.row + mv.to.row) / 2;
                self.en_passant_target = Some(Position::new(en_passant_row, mv.from.col).unwrap());
            } else {
                self.en_passant_target = None;
            }

            if let Some(promotion) = mv.promotion {
                self.board[mv.to.row][mv.to.col] = Some(promotion);
                self.current_turn = self.current_turn.opposite();
                return;
            }
        } else {
            self.en_passant_target = None;
        }

        if !is_en_passant {
            self.board[mv.to.row][mv.to.col] = None;
        }

        self.board[mv.to.row][mv.to.col] = Some(piece);
        self.current_turn = self.current_turn.opposite();
    }

    pub fn is_in_check(&self, color: Color) -> bool {
        let king_pos = self.find_king(color);
        self.is_square_attacked(king_pos, color.opposite())
    }

    pub fn is_checkmate(&self) -> bool {
        if !self.is_in_check(self.current_turn) {
            return false;
        }

        for row in 0..8 {
            for col in 0..8 {
                let pos = Position::new(row, col).unwrap();
                if let Some(piece) = self.get(pos) {
                    if piece.color() == self.current_turn {
                        if !self.get_legal_moves(pos).is_empty() {
                            return false;
                        }
                    }
                }
            }
        }

        true
    }

    pub fn is_stalemate(&self) -> bool {
        if self.is_in_check(self.current_turn) {
            return false;
        }

        for row in 0..8 {
            for col in 0..8 {
                let pos = Position::new(row, col).unwrap();
                if let Some(piece) = self.get(pos) {
                    if piece.color() == self.current_turn {
                        if !self.get_legal_moves(pos).is_empty() {
                            return false;
                        }
                    }
                }
            }
        }

        true
    }

    fn find_king(&self, color: Color) -> Position {
        for row in 0..8 {
            for col in 0..8 {
                if let Some(Piece::King(king_color, _)) = self.board[row][col] {
                    if king_color == color {
                        return Position { row, col };
                    }
                }
            }
        }
        panic!("King not found!");
    }

    fn is_square_attacked(&self, pos: Position, by_color: Color) -> bool {
        // 检查被马攻击
        let knight_moves = [
            (-2, -1),
            (-2, 1),
            (-1, -2),
            (-1, 2),
            (1, -2),
            (1, 2),
            (2, -1),
            (2, 1),
        ];

        for &(dr, dc) in &knight_moves {
            let new_row = pos.row as i32 + dr;
            let new_col = pos.col as i32 + dc;

            if new_row >= 0 && new_row < 8 && new_col >= 0 && new_col < 8 {
                if let Some(Piece::Knight(color)) = self.board[new_row as usize][new_col as usize] {
                    if color == by_color {
                        return true;
                    }
                }
            }
        }

        // 检查被兵攻击
        let pawn_direction = match by_color {
            Color::White => 1,
            Color::Black => -1,
        };

        for &dc in &[-1, 1] {
            let new_row = pos.row as i32 + pawn_direction;
            let new_col = pos.col as i32 + dc;

            if new_row >= 0 && new_row < 8 && new_col >= 0 && new_col < 8 {
                if let Some(Piece::Pawn(color, _)) = self.board[new_row as usize][new_col as usize]
                {
                    if color == by_color {
                        return true;
                    }
                }
            }
        }

        // 检查被滑动棋子攻击
        let sliding_directions = [
            (-1, -1),
            (-1, 1),
            (1, -1),
            (1, 1),
            (-1, 0),
            (1, 0),
            (0, -1),
            (0, 1),
        ];

        for &(dr, dc) in &sliding_directions {
            let mut new_row = pos.row as i32 + dr;
            let mut new_col = pos.col as i32 + dc;

            while new_row >= 0 && new_row < 8 && new_col >= 0 && new_col < 8 {
                let new_row_usize = new_row as usize;
                let new_col_usize = new_col as usize;

                if let Some(piece) = self.board[new_row_usize][new_col_usize] {
                    if piece.color() == by_color {
                        match piece {
                            Piece::Queen(_) => return true,
                            Piece::Rook(_, _) if dr == 0 || dc == 0 => return true,
                            Piece::Bishop(_) if dr != 0 && dc != 0 => return true,
                            _ => (),
                        }
                    }
                    break;
                }
                new_row += dr;
                new_col += dc;
            }
        }

        // 检查被王攻击
        let king_moves = [
            (-1, -1),
            (-1, 0),
            (-1, 1),
            (0, -1),
            (0, 1),
            (1, -1),
            (1, 0),
            (1, 1),
        ];

        for &(dr, dc) in &king_moves {
            let new_row = pos.row as i32 + dr;
            let new_col = pos.col as i32 + dc;

            if new_row >= 0 && new_row < 8 && new_col >= 0 && new_col < 8 {
                if let Some(Piece::King(color, _)) = self.board[new_row as usize][new_col as usize]
                {
                    if color == by_color {
                        return true;
                    }
                }
            }
        }

        false
    }

    pub fn display(&self) {
        println!("  a b c d e f g h");
        println!("  ----------------");

        for row in 0..8 {
            print!("{}|", 8 - row);
            for col in 0..8 {
                let symbol = match self.board[row][col] {
                    Some(Piece::King(Color::White, _)) => "♔",
                    Some(Piece::Queen(Color::White)) => "♕",
                    Some(Piece::Rook(Color::White, _)) => "♖",
                    Some(Piece::Bishop(Color::White)) => "♗",
                    Some(Piece::Knight(Color::White)) => "♘",
                    Some(Piece::Pawn(Color::White, _)) => "♙",
                    Some(Piece::King(Color::Black, _)) => "♚",
                    Some(Piece::Queen(Color::Black)) => "♛",
                    Some(Piece::Rook(Color::Black, _)) => "♜",
                    Some(Piece::Bishop(Color::Black)) => "♝",
                    Some(Piece::Knight(Color::Black)) => "♞",
                    Some(Piece::Pawn(Color::Black, _)) => "♟",
                    None => " ",
                };
                print!("{}", symbol);
                if col < 7 {
                    print!(" ");
                }
            }
            println!("|{}", 8 - row);
        }

        println!("  ----------------");
        println!("  a b c d e f g h");
        println!("当前回合: {}", self.current_turn);

        if self.is_in_check(self.current_turn) {
            println!("{}被将军!", self.current_turn);
        }
    }

    pub fn display_move_history(&self) {
        println!("移动历史:");
        for (i, mv) in self.move_history.iter().enumerate() {
            println!("{}. {}", i + 1, mv);
        }
    }
}

fn handle_promotion(color: Color) -> Piece {
    println!("兵升变! 请选择升变的棋子:");
    println!("1. 后 (Q)");
    println!("2. 车 (R)");
    println!("3. 象 (B)");
    println!("4. 马 (N)");

    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("读取输入失败");

    match input.trim() {
        "1" | "Q" | "q" => Piece::Queen(color),
        "2" | "R" | "r" => Piece::Rook(color, true),
        "3" | "B" | "b" => Piece::Bishop(color),
        "4" | "N" | "n" => Piece::Knight(color),
        _ => {
            println!("无效选择，默认升变为后");
            Piece::Queen(color)
        }
    }
}

#[tokio::main] // 正确：使用Tokio宏包装同步main函数
async fn main() {
    let mut board = Chessboard::new();
    let ai_client = SiliconFlowClient::new(
        env::var("SILICON_FLOW_API_KEY").expect("请设置环境变量 SILICON_FLOW_API_KEY"),
    );

    println!("欢迎来到国际象棋!");
    println!("输入格式: 起始位置 目标位置 (例如: e2 e4)");
    println!("特殊命令:");
    println!("  'history' - 显示移动历史");
    println!("  'quit' - 退出游戏");
    println!("  'help' - 显示帮助");

    loop {
        board.display();

        if board.is_checkmate() {
            println!("将死! {}获胜!", board.current_turn().opposite());
            break;
        }

        if board.is_stalemate() {
            println!("僵局! 游戏平局!");
            break;
        }

        let mv = if board.current_turn() == Color::Black {
            // AI回合
            println!("AI思考中...");
            let fen = board.to_fen();

            match ai_client.get_best_move(&fen).await {
                Ok(move_from_api) => move_from_api,
                Err(e) => {
                    println!("API调用失败: {:?}, 使用备用AI", e);
                    board.get_random_legal_move().expect("无合法走法")
                }
            }
        } else {
            // 玩家回合
            println!("\n{}的回合，请输入移动:", board.current_turn());

            let mut input = String::new();
            io::stdin().read_line(&mut input).expect("读取输入失败");
            let input = input.trim();

            match input {
                "quit" | "exit" => {
                    println!("游戏结束!");
                    break;
                }
                "history" => {
                    board.display_move_history();
                    continue;
                }
                "help" => {
                    println!("输入格式: 起始位置 目标位置 (例如: e2 e4)");
                    println!("特殊命令:");
                    println!("  'history' - 显示移动历史");
                    println!("  'quit' - 退出游戏");
                    println!("  'help' - 显示帮助");
                    continue;
                }
                _ => {}
            }

            let mut mv = match Move::from_notation(input) {
                Some(mv) => mv,
                None => {
                    println!("无效的移动格式，请使用格式: e2 e4");
                    continue;
                }
            };

            // 检查是否是兵升变
            if let Some(Piece::Pawn(color, _)) = board.get(mv.from) {
                let promotion_row = match color {
                    Color::White => 0,
                    Color::Black => 7,
                };
                if mv.to.row == promotion_row {
                    let promotion_piece = handle_promotion(color);
                    mv.promotion = Some(promotion_piece);
                }
            }

            mv
        };

        match board.make_move(&mv) {
            Ok(_) => println!("移动成功: {}", mv.to_notation()),
            Err(e) => {
                println!("移动失败: {}", e);
                if board.current_turn() == Color::Black {
                    // AI走法非法时使用备用随机走法
                    println!("AI走法非法，使用备用随机走法");
                    let backup_move = board.get_random_legal_move().expect("无合法走法");
                    board.make_move(&backup_move).unwrap();
                }
            }
        }
    } // 游戏主循环结束（loop {} 闭合）

    // 游戏结束后显示移动历史
    board.display_move_history();
    println!("感谢游戏!");
}
