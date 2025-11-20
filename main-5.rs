use bevy::prelude::*;
use bevy_tweening::*;

// 棋盘属性（8x8格子，单个格子尺寸）
#[derive(Component)]
struct Chessboard {
    cell_size: f32,  // 单个格子像素尺寸（如100.0）
}

// 棋子类型（王/后/车/象/马/兵）
#[derive(Debug, Clone, Copy, PartialEq)]
enum PieceType {
    King, Queen, Rook, Bishop, Knight, Pawn
}

// 棋子颜色（黑/白）
#[derive(Debug, Clone, Copy, PartialEq)]
enum PieceColor {
    White, Black
}

// 棋子组件（关联类型、颜色、位置）
#[derive(Component)]
struct Piece {
    piece_type: PieceType,
    color: PieceColor,
    position: (u8, u8),  // (行, 列)，范围0-7（对应棋盘8x8）
}

// 拖放状态组件（标记是否正在拖动）
#[derive(Component)]
struct Dragging {
    start_position: Vec3,  // 拖动起始位置
}

// 动画组件（用于棋子移动/消失动画）
#[derive(Component)]
struct PieceAnimation(Tween<Transform>);
/// 初始化棋盘
fn setup_board(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    let cell_size = 100.0;  // 每个格子100x100像素
    let board_size = cell_size * 8.0;  // 棋盘总尺寸800x800

    // 生成8x8格子
    for row in 0..8 {
        for col in 0..8 {
            // 交替颜色（白/棕）
            let color = if (row + col) % 2 == 0 {
                Color::rgb(0.9, 0.9, 0.9)  // 白色格子
            } else {
                Color::rgb(0.5, 0.3, 0.1)  // 棕色格子
            };

            // 计算格子位置（原点在屏幕中心，棋盘居中）
            let x = col as f32 * cell_size - board_size / 2.0 + cell_size / 2.0;
            let y = row as f32 * cell_size - board_size / 2.0 + cell_size / 2.0;

            // 生成格子实体（2D矩形）
            commands.spawn(SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::new(cell_size, cell_size)),
                    ..default()
                },
                transform: Transform::from_xyz(x, y, 0.0),  // z=0（底层）
                material: materials.add(color.into()),
                ..default()
            });
        }
    }

    // 生成棋盘根实体（存储属性）
    commands.spawn((
        Chessboard { cell_size },
        Transform::from_xyz(0.0, 0.0, 0.0),  // 棋盘居中
        GlobalTransform::default(),
    ));
}
// 棋子纹理资源（存储所有棋子的图片句柄）
#[derive(Resource)]
struct PieceTextures {
    white_king: Handle<Image>,
    white_queen: Handle<Image>,
    // ... 其他白棋类型
    black_king: Handle<Image>,
    black_queen: Handle<Image>,
    // ... 其他黑棋类型
}

/// 加载棋子纹理资源
fn load_piece_textures(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(PieceTextures {
        white_king: asset_server.load("textures/white_king.png"),
        white_queen: asset_server.load("textures/white_queen.png"),
        // ... 补充其他棋子纹理路径
        black_king: asset_server.load("textures/black_king.png"),
        black_queen: asset_server.load("textures/black_queen.png"),
        // ... 补充其他棋子纹理路径
    });
}

/// 初始化棋子（按国际象棋初始位置放置）
fn setup_pieces(
    mut commands: Commands,
    board: Query<&Chessboard>,
    textures: Res<PieceTextures>,
) {
    let board = board.single();
    let cell_size = board.cell_size;
    let board_size = cell_size * 8.0;

    // 白方后排（row=0）：车、马、象、后、王、象、马、车
    let white_back_row = [
        (PieceType::Rook, 0, 0),
        (PieceType::Knight, 0, 1),
        (PieceType::Bishop, 0, 2),
        (PieceType::Queen, 0, 3),
        (PieceType::King, 0, 4),
        (PieceType::Bishop, 0, 5),
        (PieceType::Knight, 0, 6),
        (PieceType::Rook, 0, 7),
    ];
    // 白方兵（row=1）
    let white_pawns: Vec<_> = (0..8).map(|col| (PieceType::Pawn, 1, col)).collect();

    // 黑方后排（row=7）和兵（row=6）类似，略...

    // 生成白方棋子
    for (piece_type, row, col) in white_back_row.into_iter().chain(white_pawns) {
        spawn_piece(
            &mut commands,
            piece_type,
            PieceColor::White,
            (row, col),
            cell_size,
            board_size,
            &textures,
        );
    }
}

/// 生成单个棋子实体
fn spawn_piece(
    commands: &mut Commands,
    piece_type: PieceType,
    color: PieceColor,
    position: (u8, u8),
    cell_size: f32,
    board_size: f32,
    textures: &PieceTextures,
) {
    // 根据类型和颜色获取纹理
    let texture = match (color, piece_type) {
        (PieceColor::White, PieceType::King) => textures.white_king.clone(),
        (PieceColor::White, PieceType::Queen) => textures.white_queen.clone(),
        // ... 补充其他类型映射
        _ => panic!("未定义的棋子纹理"),
    };

    // 计算棋子位置（居中于格子）
    let (row, col) = position;
    let x = col as f32 * cell_size - board_size / 2.0 + cell_size / 2.0;
    let y = row as f32 * cell_size - board_size / 2.0 + cell_size / 2.0;

    // 生成棋子实体（Sprite + Piece组件）
    commands.spawn((
        SpriteBundle {
            texture,
            sprite: Sprite {
                custom_size: Some(Vec2::new(cell_size * 0.8, cell_size * 0.8)),  // 棋子比格子小20%
                ..default()
            },
            transform: Transform::from_xyz(x, y, 1.0),  // z=1（在棋盘上方）
            ..default()
        },
        Piece { piece_type, color, position },
    ));
}
/// 处理拖动开始（鼠标按下时）
fn start_drag(
    mut commands: Commands,
    mouse_btn_input: Res<Input<MouseButton>>,
    cursor_pos: Res<CursorPosition>,  // 需要手动实现的光标位置资源
    mut pieces: Query<(Entity, &mut Transform, &Piece)>,
) {
    if mouse_btn_input.just_pressed(MouseButton::Left) {
        if let Some(cursor_world_pos) = cursor_pos.0 {  // 光标世界坐标（需转换屏幕->世界）
            // 检测鼠标是否点击了棋子（简化：距离判断）
            for (entity, transform, _) in &mut pieces {
                let distance = transform.translation.distance(cursor_world_pos);
                if distance < 50.0 {  // 假设棋子半径50像素内视为点击
                    // 标记为正在拖动
                    commands.entity(entity).insert(Dragging {
                        start_position: transform.translation.clone(),
                    });
                    // 提升z轴层级（避免被其他棋子遮挡）
                    transform.translation.z = 2.0;
                    break;
                }
            }
        }
    }
}

/// 处理拖动中（鼠标移动时）
fn drag_move(
    cursor_pos: Res<CursorPosition>,
    mut dragging_pieces: Query<&mut Transform, With<Dragging>>,
) {
    if let Some(cursor_world_pos) = cursor_pos.0 {
        for mut transform in &mut dragging_pieces {
            // 棋子跟随鼠标（保持z轴不变）
            transform.translation.x = cursor_world_pos.x;
            transform.translation.y = cursor_world_pos.y;
        }
    }
}

/// 处理拖动结束（鼠标释放时）
fn end_drag(
    mut commands: Commands,
    mouse_btn_input: Res<Input<MouseButton>>,
    board: Query<&Chessboard>,
    mut dragging_pieces: Query<(Entity, &mut Transform, &Piece, &Dragging)>,
) {
    if mouse_btn_input.just_released(MouseButton::Left) {
        let board = board.single();
        let cell_size = board.cell_size;
        let board_size = cell_size * 8.0;

        for (entity, mut transform, piece, dragging) in &mut dragging_pieces {
            // 计算鼠标释放位置对应的棋盘格子（行/列）
            let target_col = ((transform.translation.x + board_size / 2.0) / cell_size).round() as u8;
            let target_row = ((transform.translation.y + board_size / 2.0) / cell_size).round() as u8;
            let target_pos = (target_row.clamp(0, 7), target_col.clamp(0, 7));  // 限制在棋盘内

            // 检查移动是否合法（简化：仅示例，需对接国际象棋规则）
            let is_valid = true;  // 实际需根据棋子类型/颜色判断

            if is_valid {
                // 移动到目标格子（触发动画）
                let target_x = target_col as f32 * cell_size - board_size / 2.0 + cell_size / 2.0;
                let target_y = target_row as f32 * cell_size - board_size / 2.0 + cell_size / 2.0;
                start_move_animation(&mut commands, entity, transform.translation, Vec3::new(target_x, target_y, 1.0));
            } else {
                // 非法移动，回到起始位置（触发动画）
                start_move_animation(&mut commands, entity, transform.translation, dragging.start_position);
            }

            // 移除拖动状态，恢复z轴
            commands.entity(entity).remove::<Dragging>();
            transform.translation.z = 1.0;
        }
    }
}

/// 辅助函数：开始移动动画
fn start_move_animation(commands: &mut Commands, entity: Entity, start: Vec3, end: Vec3) {
    // 使用bevy_tweening创建位置插值动画（0.3秒线性移动）
    let tween = Tween::new(
        EaseFunction::Linear,
        Duration::from_secs_f32(0.3),
        TransformPositionLens { start, end },
    );
    commands.entity(entity).insert(PieceAnimation(tween));
}
/// 驱动棋子动画
fn run_animations(
    mut query: Query<(&mut Transform, &mut PieceAnimation)>,
    time: Res<Time>,
) {
    for (mut transform, mut animation) in &mut query {
        // 更新动画进度
        let _ = animation.0.update(time.delta());
        // 应用动画到Transform
        animation.0.apply(&mut transform);
        // 动画结束后移除组件
        if animation.0.finished() {
            // 可选：更新棋子位置属性（Piece.position）
        }
    }
}

/// 选中棋子时高亮格子（示例）
fn highlight_selected(
    mut commands: Commands,
    selected_piece: Query<&Piece, With<Dragging>>,  // 仅高亮正在拖动的棋子原位置
    board: Query<&Chessboard>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // 清除之前的高亮
    // ...

    if let Ok(piece) = selected_piece.get_single() {
        let (row, col) = piece.position;
        let board = board.single();
        let cell_size = board.cell_size;
        let board_size = cell_size * 8.0;

        // 计算高亮位置（原格子上方，半透明绿色）
        let x = col as f32 * cell_size - board_size / 2.0 + cell_size / 2.0;
        let y = row as f32 * cell_size - board_size / 2.0 + cell_size / 2.0;

        commands.spawn(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(cell_size, cell_size)),
                ..default()
            },
            transform: Transform::from_xyz(x, y, 0.5),  // z=0.5（在棋盘和棋子之间）
            material: materials.add(Color::rgba(0.2, 0.8, 0.2, 0.3).into()),  // 半透明绿
            ..default()
        });
    }
}
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                title: "国际象棋".to_string(),
                width: 800.0,
                height: 800.0,
                ..default()
            },
            ..default()
        }))
        .add_plugin(TweeningPlugin)  // 动画插件
        .insert_resource(CursorPosition(None))  // 光标位置资源（需实现更新逻辑）
        // 初始化系统
        .add_startup_system(setup_board)
        .add_startup_system(load_piece_textures)
        .add_startup_system(setup_pieces.after(load_piece_textures))
        // 交互系统
        .add_system(update_cursor_position)  // 需实现：屏幕坐标转世界坐标
        .add_system(start_drag)
        .add_system(drag_move)
        .add_system(end_drag)
        // 动画系统
        .add_system(run_animations)
        .add_system(highlight_selected)
        .run();
}