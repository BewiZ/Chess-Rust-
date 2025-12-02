use eframe::egui;
use egui::{
    pos2, 
    Color32, 
    Context,
    Rect, 
    Response, 
    Sense, 
    TextStyle,
    Ui,
    Vec2,
    RichText,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fmt
};
use tokio::runtime::Runtime;
use rand::Rng;

// ========== 核心数据结构（完整移动逻辑） ==========
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn opposite(&self) -> Self {
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

    // 转换为Unicode字符（用于GUI显示）
    pub fn to_unicode(&self) -> char {
        match self {
            Piece::King(Color::White, _) => '\u{2654}', // ♔
            Piece::Queen(Color::White) => '\u{2655}', // ♕
            Piece::Rook(Color::White, _) => '\u{2656}', // ♖
            Piece::Bishop(Color::White) => '\u{2657}', // ♗
            Piece::Knight(Color::White) => '\u{2658}', // ♘
            Piece::Pawn(Color::White, _) => '\u{2659}', // ♙
            Piece::King(Color::Black, _) => '\u{265A}', // ♚
            Piece::Queen(Color::Black) => '\u{265B}', // ♛
            Piece::Rook(Color::Black, _) => '\u{265C}', // ♜
            Piece::Bishop(Color::Black) => '\u{265D}', // ♝
            Piece::Knight(Color::Black) => '\u{265E}', // ♞
            Piece::Pawn(Color::Black, _) => '\u{265F}', // ♟
        }
    }

    // 获取棋子颜色（用于GUI绘制）
    pub fn draw_color(&self) -> Color32 {
        match self.color() {
            Color::White => Color32::WHITE,
            Color::Black => Color32::BLACK,
        }
    }
}

pub type Square = Option<Piece>;

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

    // 适配GUI坐标转换（点击坐标转棋盘位置）
    pub fn from_click(rel_x: f32, rel_y: f32, cell_size: f32) -> Option<Self> {
        let col = (rel_x / cell_size) as usize;
        let row = (rel_y / cell_size) as usize;
        Self::new(row, col)
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

    // 适配原有API的Move格式转换
    pub fn from_api_notation(notation: &str) -> Option<Self> {
        if notation.len() < 4 || notation.len() > 5 {
            return None;
        }
        let from_str = &notation[0..2];
        let to_str = &notation[2..4];
        let promotion = if notation.len() == 5 {
            match notation.chars().nth(4)? {
                'q' => Some(Piece::Queen(Color::Black)), // 临时默认黑方，实际会根据走法修正
                'r' => Some(Piece::Rook(Color::Black, false)),
                'b' => Some(Piece::Bishop(Color::Black)),
                'n' => Some(Piece::Knight(Color::Black)),
                _ => None,
            }
        } else {
            None
        };

        Some(Move {
            from: Position::from_notation(from_str)?,
            to: Position::from_notation(to_str)?,
            promotion,
        })
    }
}

// ========== AI客户端（原有逻辑） ==========
#[derive(Debug, Serialize, Clone)]
struct AiRequest {
    fen: String,
    depth: Option<u8>,
}

#[derive(Debug, Deserialize)]
struct AiResponse {
    best_move: String,
}

#[derive(Debug, Clone)]
pub struct SiliconFlowClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl SiliconFlowClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://api.siliconflow.com/v1/chess/analyze".to_string(),
        }
    }

    pub async fn get_best_move(&self, fen: &str) -> Result<Move, Box<dyn Error>> {
        let request = AiRequest {
            fen: fen.to_string(),
            depth: Some(8),
        };

        let response = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("API请求失败：{}", response.status()).into());
        }

        let ai_response: AiResponse = response.json().await?;
        Move::from_api_notation(&ai_response.best_move)
            .ok_or_else(|| "API返回无效走法格式".into())
    }
}

// ========== 完整棋盘逻辑（含移动规则） ==========
#[derive(Debug, Clone)]
pub struct Chessboard {
    pub board: [[Square; 8]; 8],
    pub current_turn: Color,
    pub castling_rights: CastlingRights,
    pub en_passant_target: Option<Position>,
    pub move_history: Vec<String>,
    // GUI相关字段
    pub selected_pos: Option<Position>,
    pub ai_client: SiliconFlowClient,
    pub ai_suggested_move: Option<Move>,
    pub status_message: String,
    pub show_move_history: bool,

    // 新增：待升变的移动信息 (from, to, color)
    pub pending_promotion: Option<(Position, Position, Color)>,
}

impl Chessboard {
    // 初始化棋盘（适配GUI）
    pub fn new(api_key: String) -> Self {
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

        Self {
            board,
            current_turn: Color::White,
            castling_rights: CastlingRights::new(),
            en_passant_target: None,
            move_history: Vec::new(),
            selected_pos: None,
            ai_client: SiliconFlowClient::new(api_key),
            ai_suggested_move: None,
            status_message: "游戏开始，白方先行".to_string(),
            show_move_history: false,
            pending_promotion: None, // 初始化升变等待状态
        }
    }

    // 转换为FEN格式（完整）
    pub fn to_fen(&self) -> String {
        let mut fen = String::new();

        // 棋盘布局
        for row in 0..8 {
            let mut empty = 0;
            for col in 0..8 {
                match self.board[row][col] {
                    None => empty += 1,
                    Some(piece) => {
                        if empty > 0 {
                            fen.push_str(&empty.to_string());
                            empty = 0;
                        }
                        let c = match piece {
                            Piece::King(Color::White, _) => 'K',
                            Piece::Queen(Color::White) => 'Q',
                            Piece::Rook(Color::White, _) => 'R',
                            Piece::Bishop(Color::White) => 'B',
                            Piece::Knight(Color::White) => 'N',
                            Piece::Pawn(Color::White, _) => 'P',
                            Piece::King(Color::Black, _) => 'k',
                            Piece::Queen(Color::Black) => 'q',
                            Piece::Rook(Color::Black, _) => 'r',
                            Piece::Bishop(Color::Black) => 'b',
                            Piece::Knight(Color::Black) => 'n',
                            Piece::Pawn(Color::Black, _) => 'p',
                        };
                        fen.push(c);
                    }
                }
            }
            if empty > 0 {
                fen.push_str(&empty.to_string());
            }
            if row < 7 {
                fen.push('/');
            }
        }

        // 回合
        fen.push_str(match self.current_turn {
            Color::White => " w ",
            Color::Black => " b ",
        });

        // 王车易位权利
        let mut castling = String::new();
        if self.castling_rights.white_kingside {
            castling.push('K');
        }
        if self.castling_rights.white_queenside {
            castling.push('Q');
        }
        if self.castling_rights.black_kingside {
            castling.push('k');
        }
        if self.castling_rights.black_queenside {
            castling.push('q');
        }
        if castling.is_empty() {
            castling.push('-');
        }
        fen.push_str(&castling);
        fen.push(' ');

        // 吃过路兵目标
        fen.push_str(&self.en_passant_target.map(|p| p.to_notation()).unwrap_or_else(|| "-".to_string()));
        fen.push_str(" 0 1"); // 半回合/全回合（简化）

        fen
    }

    // 获取棋子
    pub fn get(&self, pos: Position) -> Square {
        self.board[pos.row][pos.col]
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

    // 随机合法走法
    pub fn get_random_legal_move(&self) -> Option<Move> {
        let mut all_legal_moves = Vec::new();

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
            // 升变选择（默认升变为后）
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
            (-2, -1), (-2, 1), (-1, -2), (-1, 2),
            (1, -2), (1, 2), (2, -1), (2, 1),
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
            (-1, -1), (-1, 1), (1, -1), (1, 1),
            (-1, 0), (1, 0), (0, -1), (0, 1),
        ];
        self.sliding_moves(from, color, &directions, moves);
    }

    // 王的移动逻辑（含王车易位）
    fn king_moves(&self, from: Position, color: Color, moves: &mut Vec<Move>) {
        let king_moves = [
            (-1, -1), (-1, 0), (-1, 1),
            (0, -1),          (0, 1),
            (1, -1),  (1, 0), (1, 1),
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

        // 短易位（王翼）
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

        // 长易位（后翼）
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

    // 滑动棋子通用逻辑
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

    // 执行移动（带合法校验）
    pub fn make_move(&mut self, mv: &Move) -> Result<(), String> {
        let legal_moves = self.get_legal_moves(mv.from);
        if !legal_moves
            .iter()
            .any(|legal_move| legal_move.from == mv.from && legal_move.to == mv.to)
        {
            return Err(format!("非法移动：{} → {}", mv.from.to_notation(), mv.to.to_notation()));
        }

        // 保存移动信息
        let move_notation = mv.to_notation();
        let from_notation = mv.from.to_notation();
        let to_notation = mv.to.to_notation();

        if let Some(promotion) = mv.promotion {
            let promotion_symbol = match promotion {
                Piece::Queen(_) => "Q",
                Piece::Rook(_, _) => "R",
                Piece::Bishop(_) => "B",
                Piece::Knight(_) => "N",
                _ => "",
            };
            self.move_history.push(format!("{} {}", move_notation, promotion_symbol));
        } else {
            self.move_history.push(move_notation);
        }

        self.make_move_unchecked(mv);
        self.status_message = format!("移动成功：{} → {}", from_notation, to_notation);
        
        // 检查将死/僵局
        if self.is_checkmate() {
            self.status_message = format!("将死！{}获胜！", self.current_turn.opposite());
        } else if self.is_stalemate() {
            self.status_message = "僵局！游戏平局！".to_string();
        } else {
            self.status_message = format!("当前回合：{}", self.current_turn);
        }

        Ok(())
    }


    // 内部执行移动（无校验）
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

    // 检查将军
    pub fn is_in_check(&self, color: Color) -> bool {
        let king_pos = self.find_king(color);
        self.is_square_attacked(king_pos, color.opposite())
    }

    // 检查将死
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

    // 检查僵局
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

    // 查找王的位置
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
        panic!("未找到王！");
    }

    // 检查格子是否被攻击
    fn is_square_attacked(&self, pos: Position, by_color: Color) -> bool {
        // 马攻击
        let knight_moves = [
            (-2, -1), (-2, 1), (-1, -2), (-1, 2),
            (1, -2), (1, 2), (2, -1), (2, 1),
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

        // 兵攻击
        let pawn_direction = match by_color {
            Color::White => 1,
            Color::Black => -1,
        };

        for &dc in &[-1, 1] {
            let new_row = pos.row as i32 + pawn_direction;
            let new_col = pos.col as i32 + dc;

            if new_row >= 0 && new_row < 8 && new_col >= 0 && new_col < 8 {
                if let Some(Piece::Pawn(color, _)) = self.board[new_row as usize][new_col as usize] {
                    if color == by_color {
                        return true;
                    }
                }
            }
        }

        // 滑动棋子攻击
        let sliding_directions = [
            (-1, -1), (-1, 1), (1, -1), (1, 1),
            (-1, 0), (1, 0), (0, -1), (0, 1),
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

        // 王攻击
        let king_moves = [
            (-1, -1), (-1, 0), (-1, 1),
            (0, -1),          (0, 1),
            (1, -1),  (1, 0), (1, 1),
        ];

        for &(dr, dc) in &king_moves {
            let new_row = pos.row as i32 + dr;
            let new_col = pos.col as i32 + dc;

            if new_row >= 0 && new_row < 8 && new_col >= 0 && new_col < 8 {
                if let Some(Piece::King(color, _)) = self.board[new_row as usize][new_col as usize] {
                    if color == by_color {
                        return true;
                    }
                }
            }
        }

        false
    }

    // ========== GUI交互逻辑 ==========
    // 处理棋盘点击
    pub fn handle_click(&mut self, click_pos: Position) {
        // 如果有未完成的升变，直接返回（避免干扰）
        if self.pending_promotion.is_some() {
            self.status_message = "请先完成兵升变选择！".to_string();
            return;
        }
    
        // 1. 未选中棋子：选中当前回合的棋子
        if self.selected_pos.is_none() {
            if let Some(piece) = self.get(click_pos) {
                if piece.color() == self.current_turn {
                    self.selected_pos = Some(click_pos);
                    self.status_message = format!("选中：{} ({})", click_pos.to_notation(), piece.name());
                } else {
                    self.status_message = format!("只能选择{}的棋子！", self.current_turn);
                }
            } else {
                self.status_message = "该位置无棋子！".to_string();
            }
            return;
        }

        // 2. 已选中棋子：处理移动
        let selected = self.selected_pos.unwrap();
        if selected == click_pos {
            // 取消选中
            self.selected_pos = None;
            self.status_message = "取消选中".to_string();
            return;
        }

        // 执行移动
        let mv = Move {
            from: selected,
            to: click_pos,
            promotion: None, // 兵升变默认后，可扩展GUI选择
        };

        // 先检查是否是合法移动（不执行）
        let legal_moves = self.get_legal_moves(selected);
        if !legal_moves.iter().any(|m| m.from == selected && m.to == click_pos) {
            self.status_message = format!("非法移动：{} → {}", selected.to_notation(), click_pos.to_notation());
            return;
        }

        // 检查是否是兵升变
        if let Some(Piece::Pawn(color, _)) = self.get(selected) {
            let promotion_row = match color {
                Color::White => 0,
                Color::Black => 7,
            };
            if click_pos.row == promotion_row {
                // 标记待升变，暂停执行移动
                self.pending_promotion = Some((selected, click_pos, color));
                self.status_message = format!(
                    "兵升变！请选择升变的棋子：{} → {}",
                    selected.to_notation(),
                    click_pos.to_notation()
                );
                self.selected_pos = None; // 清空选中状态
                return;
            }
        }
        // 非升变移动，直接执行
        match self.make_move(&mv) {
            Ok(_) => self.selected_pos = None,
            Err(e) => self.status_message = e,
        }
    }

    /// 执行兵升变移动
    pub fn execute_promotion(&mut self, promotion_piece: Piece) {
        if let Some((from, to, _)) = self.pending_promotion.take() {
            let mv = Move {
                from,
                to,
                promotion: Some(promotion_piece),
            };

            match self.make_move(&mv) {
                Ok(_) => {
                    self.status_message = format!(
                        "兵升变成功！{} → {} ({})",
                        from.to_notation(),
                        to.to_notation(),
                        promotion_piece.name()
                    );
                    // 检查将死/僵局
                    if self.is_checkmate() {
                        self.status_message = format!("将死！{}获胜！", self.current_turn.opposite());
                    } else if self.is_stalemate() {
                        self.status_message = "僵局！游戏平局！".to_string();
                    }
                }
                Err(e) => {
                    self.status_message = format!("升变失败：{}", e);
                    self.pending_promotion = Some((from, to, promotion_piece.color())); // 保留升变状态，允许重新选择
                }
            }
        } else {
            self.status_message = "无待升变的兵！".to_string();
        }
    }

    // 获取AI推荐走法
    pub fn get_ai_move(&mut self) {
        let fen = self.to_fen();
        let rt = Runtime::new().unwrap();
        let ai_client = self.ai_client.clone();
        
        match rt.block_on(ai_client.get_best_move(&fen)) {
            Ok(mv) => {
                self.ai_suggested_move = Some(mv.clone());
                self.status_message = format!(
                    "AI推荐走法：{} → {}",
                    mv.from.to_notation(),
                    mv.to.to_notation()
                );
            }
            Err(e) => {
                self.status_message = format!("AI请求失败：{}，使用随机走法", e);
                // 生成并立即执行随机走法
                if let Some(random_move) = self.get_random_legal_move() {
                    self.ai_suggested_move = Some(random_move);
                    self.execute_ai_move(); // 强制执行随机走法
                } else {
                    self.status_message = "无可用走法，游戏结束".to_string();
                }
            }
        }
    }

    // 执行AI推荐的走法
    pub fn execute_ai_move(&mut self) {
        if let Some(mv) = self.ai_suggested_move.take() {
            match self.make_move(&mv) {
                Ok(_) => (),
                Err(e) => {
                    self.status_message = format!("AI走法非法：{}，使用随机走法", e);
                    if let Some(backup_move) = self.get_random_legal_move() {
                        self.make_move(&backup_move).unwrap();
                    }
                }
            }
        } else {
            self.status_message = "暂无AI推荐走法！".to_string();
        }
    }

    // 绘制棋盘（GUI）
    pub fn draw_board(&mut self, ui: &mut Ui, ctx: &Context) -> Response {
        const BOARD_SIZE: f32 = 600.0;
        const CELL_SIZE: f32 = BOARD_SIZE / 8.0;
        
        // 配置字体
        let mut small_font = ctx.style().text_styles.get(&TextStyle::Body).unwrap().clone();
        small_font.size = 14.0;
        
        let mut chess_font = small_font.clone();
        chess_font.size = CELL_SIZE * 0.8;

        // 分配绘制区域
        let (response, painter) = ui.allocate_painter(
            Vec2::new(BOARD_SIZE, BOARD_SIZE),
            Sense::click(),
        );
        let rect = response.rect;
        let origin = rect.min;

        // 1. 绘制斑马纹棋盘
        let light_color = Color32::from_rgb(240, 217, 181);
        let dark_color = Color32::from_rgb(181, 136, 99);
        for row in 0..8 {
            for col in 0..8 {
                let x = origin.x + col as f32 * CELL_SIZE;
                let y = origin.y + row as f32 * CELL_SIZE;
                let cell_rect = Rect::from_min_size(pos2(x, y), Vec2::splat(CELL_SIZE));
                
                let color = if (row + col) % 2 == 0 { light_color } else { dark_color };
                painter.rect_filled(cell_rect, 0.0, color);

                // 绘制坐标标注
                if row == 7 {
                    let col_label = (b'a' + col as u8) as char;
                    painter.text(
                        pos2(x + CELL_SIZE/2.0, y + CELL_SIZE - 8.0),
                        egui::Align2::CENTER_BOTTOM,
                        col_label.to_string(),
                        small_font.clone(),
                        Color32::BLACK,
                    );
                }
                if col == 0 {
                    let row_label = (8 - row).to_string();
                    painter.text(
                        pos2(x + 8.0, y + CELL_SIZE/2.0),
                        egui::Align2::LEFT_CENTER,
                        row_label,
                        small_font.clone(),
                        Color32::BLACK,
                    );
                }
            }
        }

        // 2. 绘制选中棋子高亮框
        if let Some(selected) = self.selected_pos {
            let x = origin.x + selected.col as f32 * CELL_SIZE;
            let y = origin.y + selected.row as f32 * CELL_SIZE;
            let selected_rect = Rect::from_min_size(pos2(x, y), Vec2::splat(CELL_SIZE));
            painter.rect_stroke(selected_rect, 0.0, egui::Stroke::new(4.0, Color32::RED), egui::StrokeKind::Inside);
        }

        // 3. 绘制AI推荐走法标记
        if let Some(ai_move) = self.ai_suggested_move.take() {
            // 起点（黄色）
            let from_x = origin.x + ai_move.from.col as f32 * CELL_SIZE;
            let from_y = origin.y + ai_move.from.row as f32 * CELL_SIZE;
            let from_rect = Rect::from_min_size(pos2(from_x, from_y), Vec2::splat(CELL_SIZE));
            painter.rect_stroke(from_rect, 0.0, egui::Stroke::new(3.0, Color32::YELLOW), egui::StrokeKind::Inside);
            
            // 终点（绿色）
            let to_x = origin.x + ai_move.to.col as f32 * CELL_SIZE;
            let to_y = origin.y + ai_move.to.row as f32 * CELL_SIZE;
            let to_rect = Rect::from_min_size(pos2(to_x, to_y), Vec2::splat(CELL_SIZE));
            painter.rect_stroke(to_rect, 0.0, egui::Stroke::new(3.0, Color32::GREEN), egui::StrokeKind::Inside);
        }

        // 4. 绘制棋子
        for row in 0..8 {
            for col in 0..8 {
                if let Some(piece) = self.board[row][col] {
                    let x = origin.x + col as f32 * CELL_SIZE + CELL_SIZE/2.0;
                    let y = origin.y + row as f32 * CELL_SIZE + CELL_SIZE/2.0;
                    let center = pos2(x, y);
                    
                    painter.text(
                        center,
                        egui::Align2::CENTER_CENTER,
                        piece.to_unicode().to_string(),
                        chess_font.clone(),
                        piece.draw_color(),
                    );
                }
            }
        }

        // 5. 处理点击事件
        if response.clicked() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let rel_x = pointer_pos.x - origin.x;
                let rel_y = pointer_pos.y - origin.y;
                if let Some(pos) = Position::from_click(rel_x, rel_y, CELL_SIZE) {
                    self.handle_click(pos);
                }
            }
        }

        response
    }
}

// ========== eframe应用入口 ==========
struct ChessApp {
    chessboard: Chessboard,
    api_key_input: String,
}

impl Default for ChessApp {
    fn default() -> Self {
        Self {
            chessboard: Chessboard::new("".to_string()),
            api_key_input: String::new(),
        }
    }
}

impl eframe::App for ChessApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // // 设置全局深色主题 + 自定义背景色
        // ctx.set_visuals(egui::Visuals {
        //     window_fill: Color32::from_rgb(27, 27, 27),
        //     ..egui::Visuals::dark()
        // });

        let horizontal_margin = 20;
        let vertical_margin = 25;
        egui::CentralPanel::default()
            .frame(egui::Frame::default()
                .inner_margin(egui::Margin::symmetric(horizontal_margin, vertical_margin))
                .fill(Color32::from_rgb(27, 27, 27))
            )
            .show(ctx, |ui| {
                ui.heading(RichText::new("国际象棋 AI 对战").size(24.0).color(Color32::WHITE));
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("硅基流动 API Key：").size(14.0).color(Color32::WHITE));
                    let text_edit = egui::TextEdit::singleline(&mut self.api_key_input)
                        .frame(true)
                        .min_size(Vec2::new(250.0, 28.0));
                    ui.add(text_edit);
                    ui.add_space(8.0);
                    if ui.button("设置 API Key").clicked() {
                        self.chessboard.ai_client = SiliconFlowClient::new(self.api_key_input.clone());
                        self.chessboard.status_message = "API Key已更新".to_string();
                    }
                });
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // 功能按钮
                ui.horizontal(|ui| {
                    // 统一按钮样式
                    let btn_style = |text: &str| egui::Button::new(RichText::new(text).size(14.0))
                        .min_size(Vec2::new(100.0, 30.0))
                        .fill(Color32::from_rgb(50, 80, 120));

                    if ui.add(btn_style("获取AI推荐走法")).clicked() {
                        self.chessboard.get_ai_move();
                    }
                    ui.add_space(5.0);
                    if ui.add(btn_style("执行AI走法")).clicked() {
                        self.chessboard.execute_ai_move();
                    }
                    ui.add_space(5.0);
                    if ui.add(btn_style("重置棋盘")).clicked() {
                        self.chessboard = Chessboard::new(self.api_key_input.clone());
                    }
                    ui.add_space(15.0);

                    // 显示移动历史复选框（白色文字）
                    ui.checkbox(
                        &mut self.chessboard.show_move_history,
                        RichText::new("显示移动历史").size(14.0).color(Color32::WHITE)
                    );
                });
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // 状态信息 + 将军提示
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&self.chessboard.status_message)
                        .size(16.0)
                        .color(Color32::WHITE)
                        .background_color(Color32::from_rgb(40, 60, 90)));
                    
                    if self.chessboard.is_in_check(self.chessboard.current_turn) {
                        ui.add_space(10.0);
                        ui.label(RichText::new(format!("⚠️{}被将军！", self.chessboard.current_turn))
                            .size(16.0)
                            .color(Color32::RED));
                    }
                });
                ui.add_space(8.0);

                // 兵升变选择按钮
                if let Some((_, _, color)) = self.chessboard.pending_promotion {
                    ui.separator();
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.heading(RichText::new("🔹 兵升变选择 🔹").size(18.0).color(Color32::ORANGE));
                        ui.add_space(10.0);

                        let promo_btn = |text: &str| egui::Button::new(RichText::new(text).size(14.0))
                            .min_size(Vec2::new(60.0, 25.0))
                            .fill(Color32::from_rgb(50, 80, 120));

                        if ui.add(promo_btn("后 (Q)")).clicked() {
                            self.chessboard.execute_promotion(Piece::Queen(color));
                        }
                        ui.add_space(5.0);
                        if ui.add(promo_btn("车 (R)")).clicked() {
                            self.chessboard.execute_promotion(Piece::Rook(color, false));
                        }
                        ui.add_space(5.0);
                        if ui.add(promo_btn("象 (B)")).clicked() {
                            self.chessboard.execute_promotion(Piece::Bishop(color));
                        }
                        ui.add_space(5.0);
                        if ui.add(promo_btn("马 (N)")).clicked() {
                            self.chessboard.execute_promotion(Piece::Knight(color));
                        }
                    });
                    ui.add_space(8.0);
                    ui.separator();
                }

                ui.add_space(10.0);

                let width_scale = if self.chessboard.show_move_history { 0.8 } else { 1.0 };

                ui.horizontal(|ui| {

                    ui.scope_builder(
                        egui::UiBuilder::new()
                        .max_rect(egui::Rect::from_min_size(
                            ui.available_rect_before_wrap().min, 
                            egui::vec2(ui.available_width() * width_scale, 0.0)
                        )),
                        |ui| {
                            ui.vertical_centered(|ui| {
                                self.chessboard.draw_board(ui, ctx);
                            });
                        }
                    );

                    if self.chessboard.show_move_history {
                        ui.separator();

                        ui.scope_builder(
                            egui::UiBuilder::new()
                            .max_rect(egui::Rect::from_min_size(
                                ui.available_rect_before_wrap().min, 
                                egui::vec2(ui.available_width(), ui.available_height())
                            )),
                            |ui| {
                                ui.vertical(|ui| {
                                    ui.heading(RichText::new("移动历史：").size(16.0).color(Color32::WHITE));
                                    ui.separator();
                                    ui.add_space(5.0);

                                    // 滚动显示历史（避免溢出）
                                    egui::ScrollArea::vertical()
                                        .max_height(ui.available_height() - 40.0)
                                        .auto_shrink([false; 2])
                                        .show(ui, |ui| {
                                            // 处理空历史
                                            if self.chessboard.move_history.is_empty() {
                                                ui.label(RichText::new("暂无移动记录")
                                                    .size(14.0)
                                                    .color(Color32::GRAY));
                                            } else {
                                                // 逐条显示历史（更易读）
                                                for (i, mv) in self.chessboard.move_history.iter().enumerate() {
                                                    let wob = if i % 2 == 0 { "白方" } else { "黑方" };
                                                    ui.label(RichText::new(format!("{}. {} {}", i + 1, wob, mv))
                                                        .size(14.0));
                                                    ui.add_space(3.0);
                                                }
                                            }
                                        });
                                });
                            }
                        );
                    }
                })
            });
    }
}

fn main() -> Result<(), eframe::Error> {
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_resizable(true)
            .with_inner_size([1200.0, 900.0]),
        multisampling: 4,
        renderer: eframe::Renderer::Glow,
        ..eframe::NativeOptions::default()
    };

    eframe::run_native(
        "国际象棋 AI 对战",
        options,
        Box::new(|cc| {
            let mut fonts = egui::FontDefinitions::default();
            
            fonts.font_data.insert(
                "noto_sans_sc".to_owned(),
                std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                    "../asset/font/NotoSansSC-Regular.ttf"
                ))),
            );
            fonts.font_data.insert(
                "seguisym".to_owned(),
                std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                    "C:\\Windows\\Fonts\\seguisym.ttf"
                ))),
            );

            // 设置默认字体
            for (text_style, font_families) in fonts.families.iter_mut() {
                font_families.clear();
                font_families.push("noto_sans_sc".to_owned());
                font_families.push("seguisym".to_owned());
            }
            cc.egui_ctx.set_fonts(fonts);
            Ok(Box::new(ChessApp::default()))
        }),
    )
}