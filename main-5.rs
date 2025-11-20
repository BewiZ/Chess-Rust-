// main.rs
use bevy::prelude::*;
use bevy_tweening::*;
use bevy::render::camera::RenderTarget;
use bevy::window::PrimaryWindow;

// GUI技术栈决策：使用Bevy内置EGUI + 自定义UI组件
// 架构设计：ECS + 状态驱动 + 响应式UI
// 跨平台：基于Bevy原生支持，适配桌面和Web

// ========== 状态管理 ==========
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States)]
enum AppState {
    MainMenu,
    InGame,
    Paused,
    GameOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States)]
enum GamePhase {
    WhiteTurn,
    BlackTurn,
}

// ========== 资源配置 ==========
#[derive(Resource)]
struct GameAssets {
    piece_textures: PieceTextures,
    ui_font: Handle<Font>,
    button_materials: ButtonMaterials,
}

#[derive(Resource)]
struct ButtonMaterials {
    normal: Handle<ColorMaterial>,
    hovered: Handle<ColorMaterial>,
    pressed: Handle<ColorMaterial>,
}

// ========== UI组件 ==========
#[derive(Component)]
struct MainMenuUI;

#[derive(Component)]
struct GameUI;

#[derive(Component)]
struct PauseMenuUI;

#[derive(Component)]
struct UIButton {
    action: ButtonAction,
}

#[derive(Debug, Clone)]
enum ButtonAction {
    StartGame,
    ResumeGame,
    QuitGame,
    RestartGame,
}

// ========== 游戏核心组件 ==========
#[derive(Component)]
struct Chessboard {
    cell_size: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PieceType {
    King, Queen, Rook, Bishop, Knight, Pawn
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PieceColor {
    White, Black
}

#[derive(Component)]
struct Piece {
    piece_type: PieceType,
    color: PieceColor,
    position: (u8, u8),
    has_moved: bool, // 用于王车易位和兵的第一步移动
}

#[derive(Component)]
struct Dragging {
    start_position: Vec3,
    valid_moves: Vec<(u8, u8)>, // 合法移动位置
}

#[derive(Component)]
struct PieceAnimation(Tween<Transform>);

#[derive(Component)]
struct HighlightedCell;

#[derive(Resource)]
struct GameState {
    current_turn: PieceColor,
    selected_piece: Option<Entity>,
    check_state: Option<PieceColor>, // 将军状态
    game_history: Vec<GameMove>,
}

#[derive(Clone, Debug)]
struct GameMove {
    piece: PieceType,
    from: (u8, u8),
    to: (u8, u8),
    captured_piece: Option<PieceType>,
}

// ========== 光标系统 ==========
#[derive(Resource)]
struct CursorPosition(Option<Vec2>);

#[derive(Resource)]
struct CursorWorldPosition(Option<Vec3>);

// ========== 初始化系统 ==========
fn setup_game(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // 初始化相机
    commands.spawn(Camera2dBundle::default());
    
    // 加载资源
    let piece_textures = PieceTextures {
        white_king: asset_server.load("textures/white_king.png"),
        white_queen: asset_server.load("textures/white_queen.png"),
        white_rook: asset_server.load("textures/white_rook.png"),
        white_bishop: asset_server.load("textures/white_bishop.png"),
        white_knight: asset_server.load("textures/white_knight.png"),
        white_pawn: asset_server.load("textures/white_pawn.png"),
        black_king: asset_server.load("textures/black_king.png"),
        black_queen: asset_server.load("textures/black_queen.png"),
        black_rook: asset_server.load("textures/black_rook.png"),
        black_bishop: asset_server.load("textures/black_bishop.png"),
        black_knight: asset_server.load("textures/black_knight.png"),
        black_pawn: asset_server.load("textures/black_pawn.png"),
    };

    let button_materials = ButtonMaterials {
        normal: materials.add(Color::rgb(0.15, 0.15, 0.15).into()),
        hovered: materials.add(Color::rgb(0.25, 0.25, 0.25).into()),
        pressed: materials.add(Color::rgb(0.35, 0.35, 0.35).into()),
    };

    commands.insert_resource(GameAssets {
        piece_textures,
        ui_font: asset_server.load("fonts/FiraSans-Bold.ttf"),
        button_materials,
    });

    commands.insert_resource(GameState {
        current_turn: PieceColor::White,
        selected_piece: None,
        check_state: None,
        game_history: Vec::new(),
    });

    // 设置初始状态
    commands.insert_resource(NextState(Some(AppState::MainMenu)));
}

// ========== UI系统 ==========
fn setup_main_menu(
    mut commands: Commands,
    assets: Res<GameAssets>,
    button_materials: Res<ButtonMaterials>,
) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                background_color: Color::rgba(0.1, 0.1, 0.1, 0.9).into(),
                ..default()
            },
            MainMenuUI,
        ))
        .with_children(|parent| {
            // 标题
            parent.spawn(TextBundle::from_section(
                "国际象棋",
                TextStyle {
                    font: assets.ui_font.clone(),
                    font_size: 60.0,
                    color: Color::WHITE,
                },
            ));

            // 开始游戏按钮
            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            size: Size::new(Val::Px(200.0), Val::Px(65.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            margin: UiRect::all(Val::Px(20.0)),
                            ..default()
                        },
                        background_color: button_materials.normal.clone().into(),
                        ..default()
                    },
                    UIButton {
                        action: ButtonAction::StartGame,
                    },
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "开始游戏",
                        TextStyle {
                            font: assets.ui_font.clone(),
                            font_size: 30.0,
                            color: Color::WHITE,
                        },
                    ));
                });

            // 退出游戏按钮
            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            size: Size::new(Val::Px(200.0), Val::Px(65.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            margin: UiRect::all(Val::Px(20.0)),
                            ..default()
                        },
                        background_color: button_materials.normal.clone().into(),
                        ..default()
                    },
                    UIButton {
                        action: ButtonAction::QuitGame,
                    },
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "退出游戏",
                        TextStyle {
                            font: assets.ui_font.clone(),
                            font_size: 30.0,
                            color: Color::WHITE,
                        },
                    ));
                });
        });
}

fn setup_game_ui(mut commands: Commands, assets: Res<GameAssets>) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                    position_type: PositionType::Absolute,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::all(Val::Px(20.0)),
                    ..default()
                },
                ..default()
            },
            GameUI,
        ))
        .with_children(|parent| {
            // 顶部状态栏
            parent
                .spawn(NodeBundle {
                    style: Style {
                        size: Size::new(Val::Percent(100.0), Val::Px(50.0)),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: Color::rgba(0.0, 0.0, 0.0, 0.5).into(),
                    ..default()
                })
                .with_children(|parent| {
                    // 当前回合显示
                    parent.spawn(TextBundle::from_section(
                        "白方回合",
                        TextStyle {
                            font: assets.ui_font.clone(),
                            font_size: 24.0,
                            color: Color::WHITE,
                        },
                    ));

                    // 暂停按钮
                    parent
                        .spawn((
                            ButtonBundle {
                                style: Style {
                                    size: Size::new(Val::Px(100.0), Val::Px(40.0)),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                background_color: Color::DARK_GRAY.into(),
                                ..default()
                            },
                            UIButton {
                                action: ButtonAction::QuitGame, // 临时使用退出按钮
                            },
                        ))
                        .with_children(|parent| {
                            parent.spawn(TextBundle::from_section(
                                "菜单",
                                TextStyle {
                                    font: assets.ui_font.clone(),
                                    font_size: 20.0,
                                    color: Color::WHITE,
                                },
                            ));
                        });
                });
        });
}

// ========== 按钮交互系统 ==========
fn button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &UIButton),
        (Changed<Interaction>, With<Button>)
    >,
    mut app_state: ResMut<NextState<AppState>>,
    mut game_state: ResMut<NextState<GamePhase>>,
    button_materials: Res<ButtonMaterials>,
) {
    for (interaction, mut background_color, button) in &mut interaction_query {
        match *interaction {
            Interaction::Clicked => {
                *background_color = button_materials.pressed.clone().into();
                handle_button_action(button.action.clone(), &mut app_state, &mut game_state);
            }
            Interaction::Hovered => {
                *background_color = button_materials.hovered.clone().into();
            }
            Interaction::None => {
                *background_color = button_materials.normal.clone().into();
            }
        }
    }
}

fn handle_button_action(
    action: ButtonAction,
    app_state: &mut ResMut<NextState<AppState>>,
    game_state: &mut ResMut<NextState<GamePhase>>,
) {
    match action {
        ButtonAction::StartGame => {
            app_state.set(AppState::InGame);
            game_state.set(GamePhase::WhiteTurn);
        }
        ButtonAction::ResumeGame => {
            app_state.set(AppState::InGame);
        }
        ButtonAction::RestartGame => {
            // 重新开始游戏逻辑
        }
        ButtonAction::QuitGame => {
            // 退出游戏逻辑
        }
    }
}

// ========== 棋盘和棋子系统 ==========
fn setup_board(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let cell_size = 100.0;
    let board_size = cell_size * 8.0;

    // 创建棋盘背景
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(board_size + 20.0, board_size + 20.0)),
            color: Color::rgb(0.3, 0.2, 0.1),
            ..default()
        },
        transform: Transform::from_xyz(0.0, 0.0, -1.0),
        ..default()
    });

    // 创建棋盘格子
    for row in 0..8 {
        for col in 0..8 {
            let color = if (row + col) % 2 == 0 {
                Color::rgb(0.9, 0.9, 0.8)
            } else {
                Color::rgb(0.6, 0.4, 0.2)
            };

            let x = col as f32 * cell_size - board_size / 2.0 + cell_size / 2.0;
            let y = row as f32 * cell_size - board_size / 2.0 + cell_size / 2.0;

            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(cell_size, cell_size)),
                        ..default()
                    },
                    transform: Transform::from_xyz(x, y, 0.0),
                    material: materials.add(color.into()),
                    ..default()
                },
                HighlightedCell, // 用于后续高亮
            ));
        }
    }

    commands.spawn((
        Chessboard { cell_size },
        Transform::default(),
        GlobalTransform::default(),
    ));
}

fn setup_pieces(
    mut commands: Commands,
    board: Query<&Chessboard>,
    assets: Res<GameAssets>,
) {
    let board = board.single();
    let cell_size = board.cell_size;
    let board_size = cell_size * 8.0;

    // 白方棋子
    let white_pieces = [
        (PieceType::Rook, 0, 0), (PieceType::Knight, 0, 1), (PieceType::Bishop, 0, 2),
        (PieceType::Queen, 0, 3), (PieceType::King, 0, 4), (PieceType::Bishop, 0, 5),
        (PieceType::Knight, 0, 6), (PieceType::Rook, 0, 7),
    ];

    // 白方兵
    for col in 0..8 {
        spawn_piece(&mut commands, PieceType::Pawn, PieceColor::White, (1, col), cell_size, board_size, &assets);
    }

    // 黑方棋子
    let black_pieces = [
        (PieceType::Rook, 7, 0), (PieceType::Knight, 7, 1), (PieceType::Bishop, 7, 2),
        (PieceType::Queen, 7, 3), (PieceType::King, 7, 4), (PieceType::Bishop, 7, 5),
        (PieceType::Knight, 7, 6), (PieceType::Rook, 7, 7),
    ];

    // 黑方兵
    for col in 0..8 {
        spawn_piece(&mut commands, PieceType::Pawn, PieceColor::Black, (6, col), cell_size, board_size, &assets);
    }

    // 生成其他棋子
    for (piece_type, row, col) in white_pieces.iter().chain(black_pieces.iter()) {
        spawn_piece(&mut commands, *piece_type, 
                   if *row == 0 { PieceColor::White } else { PieceColor::Black }, 
                   (*row, *col), cell_size, board_size, &assets);
    }
}

fn spawn_piece(
    commands: &mut Commands,
    piece_type: PieceType,
    color: PieceColor,
    position: (u8, u8),
    cell_size: f32,
    board_size: f32,
    assets: &GameAssets,
) {
    let texture = match (color, piece_type) {
        (PieceColor::White, PieceType::King) => assets.piece_textures.white_king.clone(),
        (PieceColor::White, PieceType::Queen) => assets.piece_textures.white_queen.clone(),
        (PieceColor::White, PieceType::Rook) => assets.piece_textures.white_rook.clone(),
        (PieceColor::White, PieceType::Bishop) => assets.piece_textures.white_bishop.clone(),
        (PieceColor::White, PieceType::Knight) => assets.piece_textures.white_knight.clone(),
        (PieceColor::White, PieceType::Pawn) => assets.piece_textures.white_pawn.clone(),
        (PieceColor::Black, PieceType::King) => assets.piece_textures.black_king.clone(),
        (PieceColor::Black, PieceType::Queen) => assets.piece_textures.black_queen.clone(),
        (PieceColor::Black, PieceType::Rook) => assets.piece_textures.black_rook.clone(),
        (PieceColor::Black, PieceType::Bishop) => assets.piece_textures.black_bishop.clone(),
        (PieceColor::Black, PieceType::Knight) => assets.piece_textures.black_knight.clone(),
        (PieceColor::Black, PieceType::Pawn) => assets.piece_textures.black_pawn.clone(),
    };

    let (row, col) = position;
    let x = col as f32 * cell_size - board_size / 2.0 + cell_size / 2.0;
    let y = row as f32 * cell_size - board_size / 2.0 + cell_size / 2.0;

    commands.spawn((
        SpriteBundle {
            texture,
            sprite: Sprite {
                custom_size: Some(Vec2::new(cell_size * 0.8, cell_size * 0.8)),
                ..default()
            },
            transform: Transform::from_xyz(x, y, 2.0),
            ..default()
        },
        Piece {
            piece_type,
            color,
            position,
            has_moved: false,
        },
    ));
}

// ========== 输入和交互系统 ==========
fn update_cursor_position(
    mut cursor_pos: ResMut<CursorPosition>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
) {
    let window = window_query.single();
    let (camera, camera_transform) = camera_query.single();

    if let Some(cursor_pos_screen) = window.cursor_position() {
        if let Some(cursor_pos_world) = camera.viewport_to_world_2d(camera_transform, cursor_pos_screen) {
            cursor_pos.0 = Some(cursor_pos_world);
        }
    }
}

fn start_drag(
    mut commands: Commands,
    mouse_btn_input: Res<Input<MouseButton>>,
    cursor_pos: Res<CursorPosition>,
    mut pieces: Query<(Entity, &mut Transform, &Piece)>,
    game_state: Res<GameState>,
) {
    if mouse_btn_input.just_pressed(MouseButton::Left) {
        if let Some(cursor_pos) = cursor_pos.0 {
            for (entity, mut transform, piece) in &mut pieces {
                // 只能拖动当前回合的棋子
                if piece.color != game_state.current_turn {
                    continue;
                }

                let distance = Vec2::distance(
                    transform.translation.truncate(),
                    cursor_pos
                );
                
                if distance < 40.0 {
                    // 计算合法移动（简化版）
                    let valid_moves = calculate_valid_moves(piece);
                    
                    commands.entity(entity).insert(Dragging {
                        start_position: transform.translation,
                        valid_moves,
                    });
                    
                    transform.translation.z = 3.0;
                    break;
                }
            }
        }
    }
}

fn calculate_valid_moves(piece: &Piece) -> Vec<(u8, u8)> {
    // 简化的合法移动计算
    // 实际实现需要完整的国际象棋规则
    let mut moves = Vec::new();
    let (row, col) = piece.position;

    match piece.piece_type {
        PieceType::Pawn => {
            let direction = if piece.color == PieceColor::White { 1 } else { -1 };
            let new_row = (row as i8 + direction) as u8;
            if new_row < 8 {
                moves.push((new_row, col));
            }
        }
        PieceType::Knight => {
            let knight_moves = [
                (2, 1), (2, -1), (-2, 1), (-2, -1),
                (1, 2), (1, -2), (-1, 2), (-1, -2),
            ];
            for (dr, dc) in knight_moves {
                let new_row = (row as i8 + dr) as u8;
                let new_col = (col as i8 + dc) as u8;
                if new_row < 8 && new_col < 8 {
                    moves.push((new_row, new_col));
                }
            }
        }
        _ => {} // 其他棋子类型的移动规则...
    }

    moves
}

fn drag_move(
    cursor_pos: Res<CursorPosition>,
    mut dragging_pieces: Query<&mut Transform, With<Dragging>>,
) {
    if let Some(cursor_pos) = cursor_pos.0 {
        for mut transform in &mut dragging_pieces {
            transform.translation.x = cursor_pos.x;
            transform.translation.y = cursor_pos.y;
        }
    }
}

fn end_drag(
    mut commands: Commands,
    mouse_btn_input: Res<Input<MouseButton>>,
    board: Query<&Chessboard>,
    mut dragging_pieces: Query<(Entity, &mut Transform, &Piece, &Dragging)>,
    mut game_state: ResMut<GameState>,
) {
    if mouse_btn_input.just_released(MouseButton::Left) {
        let board = board.single();
        let cell_size = board.cell_size;
        let board_size = cell_size * 8.0;

        for (entity, mut transform, piece, dragging) in &mut dragging_pieces {
            let target_col = ((transform.translation.x + board_size / 2.0) / cell_size).round() as u8;
            let target_row = ((transform.translation.y + board_size / 2.0) / cell_size).round() as u8;
            let target_pos = (target_row.clamp(0, 7), target_col.clamp(0, 7));

            let is_valid = dragging.valid_moves.contains(&target_pos);

            if is_valid {
                let target_x = target_col as f32 * cell_size - board_size / 2.0 + cell_size / 2.0;
                let target_y = target_row as f32 * cell_size - board_size / 2.0 + cell_size / 2.0;
                
                start_move_animation(&mut commands, entity, transform.translation, Vec3::new(target_x, target_y, 2.0));
                
                // 更新游戏状态
                game_state.current_turn = if game_state.current_turn == PieceColor::White {
                    PieceColor::Black
                } else {
                    PieceColor::White
                };
            } else {
                start_move_animation(&mut commands, entity, transform.translation, dragging.start_position);
            }

            commands.entity(entity).remove::<Dragging>();
            transform.translation.z = 2.0;
        }
    }
}

// ========== 动画系统 ==========
fn start_move_animation(commands: &mut Commands, entity: Entity, start: Vec3, end: Vec3) {
    let tween = Tween::new(
        EaseFunction::CubicOut,
        Duration::from_secs_f32(0.3),
        TransformPositionLens { start, end },
    );
    commands.entity(entity).insert(PieceAnimation(tween));
}

fn run_animations(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut PieceAnimation)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut animation) in &mut query {
        animation.0.tick(time.delta());
        animation.0.apply(&mut transform);
        
        if animation.0.finished() {
            commands.entity(entity).remove::<PieceAnimation>();
        }
    }
}

// ========== 状态切换系统 ==========
fn handle_state_transitions(
    mut commands: Commands,
    app_state: Res<State<AppState>>,
    mut next_app_state: ResMut<NextState<AppState>>,
) {
    // 状态进入时的清理和初始化
    match app_state.get() {
        AppState::MainMenu => {
            // 清理游戏实体
            // 初始化主菜单
        }
        AppState::InGame => {
            // 初始化游戏场景
        }
        _ => {}
    }
}

// ========== 渲染优化系统 ==========
fn optimize_rendering(
    mut query: Query<&mut Visibility, With<Piece>>,
    game_state: Res<GameState>,
) {
    // 根据游戏状态优化渲染
    // 例如：在菜单状态隐藏棋子
    for mut visibility in &mut query {
        // 根据需要进行可见性调整
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "国际象棋 - 优化版".to_string(),
                resolution: (1000.0, 800.0).into(),
                present_mode: bevy::window::PresentMode::AutoVsync,
                resizable: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugin(TweeningPlugin)
        .add_state::<AppState>()
        .add_state::<GamePhase>()
        .insert_resource(CursorPosition(None))
        .insert_resource(CursorWorldPosition(None))
        // 初始化系统
        .add_startup_system(setup_game)
        // 状态管理系统
        .add_system(handle_state_transitions)
        // UI系统
        .add_system(setup_main_menu.in_schedule(OnEnter(AppState::MainMenu)))
        .add_system(setup_game_ui.in_schedule(OnEnter(AppState::InGame)))
        .add_system(button_system)
        // 核心游戏系统
        .add_system(setup_board.in_schedule(OnEnter(AppState::InGame)))
        .add_system(setup_pieces.in_schedule(OnEnter(AppState::InGame)))
        .add_system(update_cursor_position)
        .add_system(start_drag.run_if(in_state(AppState::InGame)))
        .add_system(drag_move.run_if(in_state(AppState::InGame)))
        .add_system(end_drag.run_if(in_state(AppState::InGame)))
        .add_system(run_animations)
        // 渲染优化
        .add_system(optimize_rendering)
        .run();
}