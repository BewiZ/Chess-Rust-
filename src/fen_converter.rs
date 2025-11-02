use super::{Chessboard, Color, Piece, Position};

impl Chessboard {
    // 转换为FEN字符串
    pub fn to_fen(&self) -> String {
        let mut fen = String::new();

        // 棋盘布局
        for row in 0..8 {
            let mut empty = 0;
            for col in 0..8 {
                match self.board[row][col] {
                    Some(piece) => {
                        if empty > 0 {
                            fen.push_str(&empty.to_string());
                            empty = 0;
                        }
                        fen.push(match piece {
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
                        });
                    }
                    None => empty += 1,
                }
            }
            if empty > 0 {
                fen.push_str(&empty.to_string());
            }
            if row < 7 {
                fen.push('/');
            }
        }

        // 当前回合
        fen.push(' ');
        fen.push(if self.current_turn == Color::White {
            'w'
        } else {
            'b'
        });

        // 王车易位权限
        fen.push(' ');
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

        // 吃过路兵目标
        fen.push(' ');
        fen.push_str(&match &self.en_passant_target {
            Some(pos) => pos.to_notation(),
            None => "-".to_string(),
        });

        // 半回合计数和全回合计数（简化实现）
        fen.push_str(" 0 1");

        fen
    }
}
