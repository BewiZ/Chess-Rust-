use rand::Rng;

impl Chessboard {
    // 生成随机合法走法作为备用方案
    pub fn get_random_legal_move(&self) -> Option<Move> {
        let mut rng = rand::thread_rng();
        let mut all_moves = Vec::new();

        // 收集所有合法走法
        for row in 0..8 {
            for col in 0..8 {
                let pos = Position::new(row, col).unwrap();
                let mut moves = self.get_legal_moves(pos);
                all_moves.append(&mut moves);
            }
        }

        if all_moves.is_empty() {
            return None;
        }

        // 随机选择一个走法
        let idx = rng.gen_range(0..all_moves.len());
        Some(all_moves[idx].clone())
    }
}
