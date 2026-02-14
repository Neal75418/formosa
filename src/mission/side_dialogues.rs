//! 支線任務對話樹
//!
//! 為 6 個 Strangers & Freaks 支線任務提供分支對話，
//! 選項影響 NPC 好感度、設置劇情旗標、解鎖隱藏內容。
//!
//! 對話 ID 對照：
//! - 200/201: 檳榔西施（開始/結束）
//! - 202/203: 廟公（開始/結束）
//! - 204/205: 夜市大廚（開始/結束）
//! - 206/207: 流浪狗大叔（開始/結束）
//! - 208/209: 飆車族（開始/結束）
//! - 210/211: 陰謀論部落客（開始/結束）

use super::dialogue::*;

/// 支線任務 NPC ID（與 side_missions.rs 的 quest_giver 對應）
const NPC_BETEL_NUT_BEAUTY: u32 = 200;
const NPC_TEMPLE_KEEPER: u32 = 201;
const NPC_NIGHT_MARKET_CHEF: u32 = 202;
const NPC_STRAY_DOG_UNCLE: u32 = 203;
const NPC_STREET_RACER: u32 = 204;
const NPC_CONSPIRACY_BLOGGER: u32 = 205;

/// 註冊所有支線任務對話與 NPC 資料
pub fn register_side_dialogues(database: &mut DialogueDatabase) {
    // NPC 資料
    database.register_npc(NpcDialogueData {
        id: NPC_BETEL_NUT_BEAUTY,
        name: "檳榔西施".to_string(),
        portrait: String::new(),
        voice_style: None,
    });
    database.register_npc(NpcDialogueData {
        id: NPC_TEMPLE_KEEPER,
        name: "廟公".to_string(),
        portrait: String::new(),
        voice_style: None,
    });
    database.register_npc(NpcDialogueData {
        id: NPC_NIGHT_MARKET_CHEF,
        name: "夜市大廚".to_string(),
        portrait: String::new(),
        voice_style: None,
    });
    database.register_npc(NpcDialogueData {
        id: NPC_STRAY_DOG_UNCLE,
        name: "流浪狗大叔".to_string(),
        portrait: String::new(),
        voice_style: None,
    });
    database.register_npc(NpcDialogueData {
        id: NPC_STREET_RACER,
        name: "飆車族老大".to_string(),
        portrait: String::new(),
        voice_style: None,
    });
    database.register_npc(NpcDialogueData {
        id: NPC_CONSPIRACY_BLOGGER,
        name: "陰謀論部落客".to_string(),
        portrait: String::new(),
        voice_style: None,
    });

    // 對話樹
    database.register_dialogue(create_betel_nut_start());
    database.register_dialogue(create_betel_nut_end());
    database.register_dialogue(create_temple_start());
    database.register_dialogue(create_temple_end());
    database.register_dialogue(create_chef_start());
    database.register_dialogue(create_chef_end());
    database.register_dialogue(create_dog_uncle_start());
    database.register_dialogue(create_dog_uncle_end());
    database.register_dialogue(create_racer_start());
    database.register_dialogue(create_racer_end());
    database.register_dialogue(create_blogger_start());
    database.register_dialogue(create_blogger_end());
}

// ============================================================================
// #1 檳榔西施的煩惱
// ============================================================================

fn create_betel_nut_start() -> DialogueTree {
    let npc = DialogueSpeaker::Npc(NPC_BETEL_NUT_BEAUTY);
    let player = DialogueSpeaker::Player;
    let mut tree = DialogueTree::new(200, "檳榔西施：求助");

    tree.add_node(
        DialogueNode::new(0, npc, "大哥……你能不能幫幫我？那群流氓又來了，每天來騷擾我，生意都沒辦法做……")
            .with_emotion(SpeakerEmotion::Sad)
            .then(1),
    );

    tree.add_node(
        DialogueNode::new(1, npc, "我報過警了，但警察來的時候他們就跑，警察走了又回來……")
            .with_emotion(SpeakerEmotion::Afraid)
            .with_choice(
                DialogueChoice::simple("放心，我幫妳趕走他們", 2)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_BETEL_NUT_BEAUTY,
                        delta: 10,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("幫忙可以，有什麼好處？", 3)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_BETEL_NUT_BEAUTY,
                        delta: -5,
                    }),
            )
            .with_choice(DialogueChoice::simple("妳不能換個地方擺攤嗎？", 4)),
    );

    // 義氣路線
    tree.add_node(
        DialogueNode::new(2, npc, "真的嗎！？太好了！他們通常在附近的巷子裡，大概三個人。拜託你了！")
            .with_emotion(SpeakerEmotion::Happy)
            .with_choice(DialogueChoice::end("沒問題，交給我")),
    );

    // 談報酬路線
    tree.add_node(
        DialogueNode::new(3, npc, "我……我沒什麼錢，但事成之後一定會好好謝謝你的！")
            .with_emotion(SpeakerEmotion::Sad)
            .with_choice(DialogueChoice::simple("好吧，我去看看", 2))
            .with_choice(
                DialogueChoice::simple("算了，我沒空", 5)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_BETEL_NUT_BEAUTY,
                        delta: -10,
                    }),
            ),
    );

    // 換攤路線
    tree.add_node(
        DialogueNode::new(4, player, "妳不能換個地方擺攤嗎？")
            .then(6),
    );

    // 拒絕結束
    tree.add_node(
        DialogueNode::new(5, npc, "嗯……好吧……")
            .with_emotion(SpeakerEmotion::Sad)
            .with_choice(DialogueChoice::end("離開")),
    );

    tree.add_node(
        DialogueNode::new(6, npc, "這個攤位是我阿嬤留下來的，我不想放棄……拜託你幫幫我好嗎？")
            .with_emotion(SpeakerEmotion::Sad)
            .with_choice(
                DialogueChoice::simple("好，我去教訓他們", 2)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_BETEL_NUT_BEAUTY,
                        delta: 5,
                    }),
            )
            .with_choice(DialogueChoice::simple("抱歉，我幫不了", 5)),
    );

    tree
}

fn create_betel_nut_end() -> DialogueTree {
    let npc = DialogueSpeaker::Npc(NPC_BETEL_NUT_BEAUTY);
    let mut tree = DialogueTree::new(201, "檳榔西施：感謝");

    tree.add_node(
        DialogueNode::new(0, npc, "太感謝你了！他們應該不敢再來了吧！")
            .with_emotion(SpeakerEmotion::Happy)
            .then(1),
    );

    tree.add_node(
        DialogueNode::new(1, npc, "這是一點心意，請你一定要收下！")
            .with_emotion(SpeakerEmotion::Happy)
            .with_choice(
                DialogueChoice::simple("不客氣，舉手之勞", 2)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_BETEL_NUT_BEAUTY,
                        delta: 5,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("以後有事隨時找我", 3)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_BETEL_NUT_BEAUTY,
                        delta: 10,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("要不要一起去吃個飯？", 4)
                    .with_condition(DialogueCondition::RelationshipMin {
                        npc_id: NPC_BETEL_NUT_BEAUTY,
                        min: 20,
                    })
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_BETEL_NUT_BEAUTY,
                        delta: 15,
                    })
                    .with_consequence(DialogueConsequence::SetStoryFlag {
                        flag: "betel_nut_beauty_date".to_string(),
                        value: true,
                    }),
            ),
    );

    // 客氣結束
    tree.add_node(
        DialogueNode::new(2, npc, "你人真好！以後路過記得來買檳榔喔，算你便宜！")
            .with_emotion(SpeakerEmotion::Happy)
            .with_choice(DialogueChoice::end("好的，再見")),
    );

    // 義氣結束
    tree.add_node(
        DialogueNode::new(3, npc, "有你在真的讓人安心！對了，我認識一些人，以後有消息我都會跟你說的！")
            .with_emotion(SpeakerEmotion::Happy)
            .with_choice(
                DialogueChoice::end("好，保持聯絡")
                    .with_consequence(DialogueConsequence::SetStoryFlag {
                        flag: "betel_nut_informant".to_string(),
                        value: true,
                    }),
            ),
    );

    // 約會路線（高好感度）
    tree.add_node(
        DialogueNode::new(4, npc, "欸！？吃飯？好……好啊！我下班之後可以……")
            .with_emotion(SpeakerEmotion::Surprised)
            .with_choice(DialogueChoice::end("那就這樣說定了")),
    );

    tree
}

// ============================================================================
// #2 廟公的籤詩
// ============================================================================

fn create_temple_start() -> DialogueTree {
    let npc = DialogueSpeaker::Npc(NPC_TEMPLE_KEEPER);
    let mut tree = DialogueTree::new(202, "廟公：警告");

    tree.add_node(
        DialogueNode::new(0, npc, "施主，老夫觀你面相，最近有劫數啊……")
            .with_emotion(SpeakerEmotion::Serious)
            .then(1),
    );

    tree.add_node(
        DialogueNode::new(1, npc, "土地公有指示，你必須去三個地方化解這個劫。東邊水源地淨身、西邊大榕樹祈福、南邊山頂拜拜。")
            .with_emotion(SpeakerEmotion::Serious)
            .with_choice(
                DialogueChoice::simple("好吧，我試試看", 2)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_TEMPLE_KEEPER,
                        delta: 10,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("我才不信迷信", 3)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_TEMPLE_KEEPER,
                        delta: -5,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("這要收費嗎？", 4),
            ),
    );

    // 虔誠路線
    tree.add_node(
        DialogueNode::new(2, npc, "善哉善哉！記得到每個地方都要誠心祈禱，土地公保佑你。")
            .with_emotion(SpeakerEmotion::Happy)
            .with_choice(DialogueChoice::end("我這就出發")),
    );

    // 不信路線
    tree.add_node(
        DialogueNode::new(3, npc, "唉……年輕人不信也沒關係，但是寧可信其有。你去走一趟也不虧嘛！")
            .with_emotion(SpeakerEmotion::Sad)
            .with_choice(
                DialogueChoice::simple("好吧好吧，去就去", 2)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_TEMPLE_KEEPER,
                        delta: 5,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("不了，謝謝", 5)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_TEMPLE_KEEPER,
                        delta: -5,
                    }),
            ),
    );

    // 收費路線
    tree.add_node(
        DialogueNode::new(4, npc, "不用不用！土地公的旨意，怎麼能收錢呢？你去做就好了。")
            .with_emotion(SpeakerEmotion::Neutral)
            .with_choice(
                DialogueChoice::simple("那好，我去", 2)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_TEMPLE_KEEPER,
                        delta: 5,
                    }),
            ),
    );

    // 拒絕結束
    tree.add_node(
        DialogueNode::new(5, npc, "施主保重……")
            .with_emotion(SpeakerEmotion::Sad)
            .with_choice(DialogueChoice::end("離開")),
    );

    tree
}

fn create_temple_end() -> DialogueTree {
    let npc = DialogueSpeaker::Npc(NPC_TEMPLE_KEEPER);
    let mut tree = DialogueTree::new(203, "廟公：解劫");

    tree.add_node(
        DialogueNode::new(0, npc, "哦！施主，你身上的煞氣已經散了！土地公果然靈驗！")
            .with_emotion(SpeakerEmotion::Happy)
            .then(1),
    );

    tree.add_node(
        DialogueNode::new(1, npc, "老夫代土地公謝謝你的誠心。這個平安符送給你，保你出入平安。")
            .with_emotion(SpeakerEmotion::Happy)
            .with_choice(
                DialogueChoice::simple("謝謝廟公，受教了", 2)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_TEMPLE_KEEPER,
                        delta: 10,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("大概只是巧合吧", 3)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_TEMPLE_KEEPER,
                        delta: -10,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("以後可以常來拜拜嗎？", 4)
                    .with_condition(DialogueCondition::RelationshipMin {
                        npc_id: NPC_TEMPLE_KEEPER,
                        min: 15,
                    })
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_TEMPLE_KEEPER,
                        delta: 15,
                    })
                    .with_consequence(DialogueConsequence::SetStoryFlag {
                        flag: "temple_regular".to_string(),
                        value: true,
                    }),
            ),
    );

    // 感謝結束
    tree.add_node(
        DialogueNode::new(2, npc, "施主慢走，有空常來拜拜！")
            .with_emotion(SpeakerEmotion::Happy)
            .with_choice(DialogueChoice::end("好的，再見")),
    );

    // 不信結束
    tree.add_node(
        DialogueNode::new(3, npc, "呵呵，信不信由你。平安符還是拿著吧。")
            .with_emotion(SpeakerEmotion::Neutral)
            .with_choice(DialogueChoice::end("好吧，謝了")),
    );

    // 常客路線（高好感度）
    tree.add_node(
        DialogueNode::new(4, npc, "當然歡迎！你以後有什麼煩惱都可以來求籤，廟公幫你解！")
            .with_emotion(SpeakerEmotion::Happy)
            .with_choice(DialogueChoice::end("好的，感恩廟公")),
    );

    tree
}

// ============================================================================
// #3 夜市大廚的挑戰
// ============================================================================

fn create_chef_start() -> DialogueTree {
    let npc = DialogueSpeaker::Npc(NPC_NIGHT_MARKET_CHEF);
    let mut tree = DialogueTree::new(204, "夜市大廚：送貨");

    tree.add_node(
        DialogueNode::new(0, npc, "欸欸欸！兄弟！你看起來腳程很快！我這邊有個急單！")
            .with_emotion(SpeakerEmotion::Surprised)
            .then(1),
    );

    tree.add_node(
        DialogueNode::new(1, npc, "三個客人的蚵仔煎要送，但我的外送仔今天請假！再不送就冷掉了！拜託幫個忙！")
            .with_emotion(SpeakerEmotion::Afraid)
            .with_choice(
                DialogueChoice::simple("交給我吧！", 2)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_NIGHT_MARKET_CHEF,
                        delta: 10,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("報酬怎麼算？", 3),
            )
            .with_choice(
                DialogueChoice::simple("我又不是外送員", 4)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_NIGHT_MARKET_CHEF,
                        delta: -5,
                    }),
            ),
    );

    // 爽快路線
    tree.add_node(
        DialogueNode::new(2, npc, "讚啦！三份蚵仔煎，地址在這，限時三分鐘！拜託趕快！")
            .with_emotion(SpeakerEmotion::Happy)
            .with_choice(DialogueChoice::end("收到，馬上出發")),
    );

    // 談報酬
    tree.add_node(
        DialogueNode::new(3, npc, "送完我請你吃一份招牌蚵仔煎加大份，再給你跑腿費！這樣可以嗎？")
            .with_emotion(SpeakerEmotion::Neutral)
            .with_choice(
                DialogueChoice::simple("成交", 2)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_NIGHT_MARKET_CHEF,
                        delta: 5,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("太少了吧", 5)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_NIGHT_MARKET_CHEF,
                        delta: -5,
                    }),
            ),
    );

    // 拒絕
    tree.add_node(
        DialogueNode::new(4, npc, "唉……那我只好自己跑了……客人要罵死我了……")
            .with_emotion(SpeakerEmotion::Sad)
            .with_choice(
                DialogueChoice::simple("好啦好啦我去", 2)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_NIGHT_MARKET_CHEF,
                        delta: 5,
                    }),
            )
            .with_choice(DialogueChoice::end("抱歉啊")),
    );

    // 嫌少路線
    tree.add_node(
        DialogueNode::new(5, npc, "拜託啦！我再加一百塊！蚵仔煎快冷掉了！")
            .with_emotion(SpeakerEmotion::Afraid)
            .with_choice(
                DialogueChoice::simple("好吧，看你這麼著急", 2)
                    .with_consequence(DialogueConsequence::GiveMoney(100)),
            ),
    );

    tree
}

fn create_chef_end() -> DialogueTree {
    let npc = DialogueSpeaker::Npc(NPC_NIGHT_MARKET_CHEF);
    let mut tree = DialogueTree::new(205, "夜市大廚：感謝");

    tree.add_node(
        DialogueNode::new(0, npc, "太厲害了！全部準時送到！客人都說讚！")
            .with_emotion(SpeakerEmotion::Happy)
            .then(1),
    );

    tree.add_node(
        DialogueNode::new(1, npc, "這是你的報酬，辛苦了！以後有空常來吃喔！")
            .with_emotion(SpeakerEmotion::Happy)
            .with_choice(
                DialogueChoice::simple("舉手之勞", 2)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_NIGHT_MARKET_CHEF,
                        delta: 5,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("你的蚵仔煎真的很好吃", 3)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_NIGHT_MARKET_CHEF,
                        delta: 10,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("以後可以打折嗎？", 4)
                    .with_condition(DialogueCondition::RelationshipMin {
                        npc_id: NPC_NIGHT_MARKET_CHEF,
                        min: 15,
                    })
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_NIGHT_MARKET_CHEF,
                        delta: 10,
                    })
                    .with_consequence(DialogueConsequence::SetStoryFlag {
                        flag: "night_market_discount".to_string(),
                        value: true,
                    }),
            ),
    );

    // 客氣結束
    tree.add_node(
        DialogueNode::new(2, npc, "你人真好！下次來我多煎一份給你！")
            .with_emotion(SpeakerEmotion::Happy)
            .with_choice(DialogueChoice::end("好的，謝啦")),
    );

    // 稱讚路線
    tree.add_node(
        DialogueNode::new(3, npc, "哈哈！識貨！下次來我教你我的獨門醬料配方！")
            .with_emotion(SpeakerEmotion::Happy)
            .with_choice(
                DialogueChoice::end("一言為定")
                    .with_consequence(DialogueConsequence::SetStoryFlag {
                        flag: "chef_secret_recipe".to_string(),
                        value: true,
                    }),
            ),
    );

    // 打折路線（高好感度）
    tree.add_node(
        DialogueNode::new(4, npc, "打折？看在你今天幫了大忙的份上——以後來都半價！")
            .with_emotion(SpeakerEmotion::Happy)
            .with_choice(DialogueChoice::end("太感謝了！")),
    );

    tree
}

// ============================================================================
// #4 流浪狗大叔
// ============================================================================

fn create_dog_uncle_start() -> DialogueTree {
    let npc = DialogueSpeaker::Npc(NPC_STRAY_DOG_UNCLE);
    let mut tree = DialogueTree::new(206, "大叔：求助");

    tree.add_node(
        DialogueNode::new(0, npc, "年輕人，你看到那邊那幾個人了嗎？他們一直在欺負這些流浪狗……")
            .with_emotion(SpeakerEmotion::Angry)
            .then(1),
    );

    tree.add_node(
        DialogueNode::new(1, npc, "我老了，打不過他們。你能不能幫我教訓教訓他們？拜託了。")
            .with_emotion(SpeakerEmotion::Sad)
            .with_choice(
                DialogueChoice::simple("虐待動物的人最可惡！交給我", 2)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_STRAY_DOG_UNCLE,
                        delta: 15,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("我來處理", 3)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_STRAY_DOG_UNCLE,
                        delta: 5,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("你為什麼不自己去報警？", 4)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_STRAY_DOG_UNCLE,
                        delta: -5,
                    }),
            ),
    );

    // 熱血路線
    tree.add_node(
        DialogueNode::new(2, npc, "好孩子！這些狗狗都是有感情的，牠們不該被這樣對待！")
            .with_emotion(SpeakerEmotion::Happy)
            .with_choice(DialogueChoice::end("我這就去")),
    );

    // 淡定路線
    tree.add_node(
        DialogueNode::new(3, npc, "謝謝你，年輕人。小心一點。")
            .with_emotion(SpeakerEmotion::Neutral)
            .with_choice(DialogueChoice::end("放心")),
    );

    // 質疑路線
    tree.add_node(
        DialogueNode::new(4, npc, "報警？上次報警警察說「只是流浪狗」就不理了。牠們也是生命啊……")
            .with_emotion(SpeakerEmotion::Sad)
            .with_choice(
                DialogueChoice::simple("你說得對，我去教訓他們", 2)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_STRAY_DOG_UNCLE,
                        delta: 10,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("抱歉，我幫不了", 5)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_STRAY_DOG_UNCLE,
                        delta: -10,
                    }),
            ),
    );

    // 拒絕結束
    tree.add_node(
        DialogueNode::new(5, npc, "唉……算了……")
            .with_emotion(SpeakerEmotion::Sad)
            .with_choice(DialogueChoice::end("離開")),
    );

    tree
}

fn create_dog_uncle_end() -> DialogueTree {
    let npc = DialogueSpeaker::Npc(NPC_STRAY_DOG_UNCLE);
    let mut tree = DialogueTree::new(207, "大叔：感謝");

    tree.add_node(
        DialogueNode::new(0, npc, "太好了！那些混蛋跑了！你看，狗狗們都在搖尾巴呢！")
            .with_emotion(SpeakerEmotion::Happy)
            .then(1),
    );

    tree.add_node(
        DialogueNode::new(1, npc, "我沒什麼錢，但這個是我的一點心意。謝謝你，年輕人。")
            .with_emotion(SpeakerEmotion::Happy)
            .with_choice(
                DialogueChoice::simple("照顧好狗狗們", 2)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_STRAY_DOG_UNCLE,
                        delta: 10,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("不用客氣", 3),
            )
            .with_choice(
                DialogueChoice::simple("以後我也想來幫忙餵狗", 4)
                    .with_condition(DialogueCondition::RelationshipMin {
                        npc_id: NPC_STRAY_DOG_UNCLE,
                        min: 20,
                    })
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_STRAY_DOG_UNCLE,
                        delta: 20,
                    })
                    .with_consequence(DialogueConsequence::SetStoryFlag {
                        flag: "dog_volunteer".to_string(),
                        value: true,
                    }),
            ),
    );

    // 關心動物路線
    tree.add_node(
        DialogueNode::new(2, npc, "放心，我會的。牠們就像我的家人一樣。")
            .with_emotion(SpeakerEmotion::Happy)
            .with_choice(
                DialogueChoice::end("再見，大叔")
                    .with_consequence(DialogueConsequence::SetStoryFlag {
                        flag: "dog_uncle_friend".to_string(),
                        value: true,
                    }),
            ),
    );

    // 簡單結束
    tree.add_node(
        DialogueNode::new(3, npc, "好人有好報！")
            .with_emotion(SpeakerEmotion::Happy)
            .with_choice(DialogueChoice::end("再見")),
    );

    // 志工路線（高好感度）
    tree.add_node(
        DialogueNode::new(4, npc, "真的嗎！？太好了！你隨時都可以來！狗狗們肯定會很高興的！")
            .with_emotion(SpeakerEmotion::Surprised)
            .with_choice(DialogueChoice::end("一言為定！")),
    );

    tree
}

// ============================================================================
// #5 飆車族的賭注
// ============================================================================

fn create_racer_start() -> DialogueTree {
    let npc = DialogueSpeaker::Npc(NPC_STREET_RACER);
    let mut tree = DialogueTree::new(208, "飆車族：挑戰");

    tree.add_node(
        DialogueNode::new(0, npc, "喲！你就是最近在島上鬧很兇的那個？")
            .with_emotion(SpeakerEmotion::Smirk)
            .then(1),
    );

    tree.add_node(
        DialogueNode::new(1, npc, "我是海岸線車隊的頭。聽說你車開得不錯？來跟我比一場怎樣？有膽就來！")
            .with_emotion(SpeakerEmotion::Smirk)
            .with_choice(
                DialogueChoice::simple("來就來，誰怕誰！", 2)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_STREET_RACER,
                        delta: 10,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("賭注加碼，敢不敢？", 3)
                    .with_condition(DialogueCondition::HasMoney(1000))
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_STREET_RACER,
                        delta: 5,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("我沒興趣跟你比", 4)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_STREET_RACER,
                        delta: -10,
                    }),
            ),
    );

    // 爽快接受
    tree.add_node(
        DialogueNode::new(2, npc, "哈！有種！起點在這裡，終點在南邊的燈塔。輸的人請客！")
            .with_emotion(SpeakerEmotion::Happy)
            .with_choice(DialogueChoice::end("準備好了，開始吧")),
    );

    // 加碼路線
    tree.add_node(
        DialogueNode::new(3, npc, "喔？有膽量！好，賭注翻倍！你贏了我給你雙倍獎金。你輸了……嘿嘿。")
            .with_emotion(SpeakerEmotion::Smirk)
            .with_choice(
                DialogueChoice::simple("成交！", 2)
                    .with_consequence(DialogueConsequence::SetStoryFlag {
                        flag: "racer_double_bet".to_string(),
                        value: true,
                    }),
            )
            .with_choice(DialogueChoice::simple("算了，一般賭注就好", 2)),
    );

    // 拒絕
    tree.add_node(
        DialogueNode::new(4, npc, "切！膽小鬼。不敢就別在島上混。")
            .with_emotion(SpeakerEmotion::Angry)
            .with_choice(
                DialogueChoice::simple("你說誰膽小！我來！", 2)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_STREET_RACER,
                        delta: 15,
                    }),
            )
            .with_choice(DialogueChoice::end("隨便你")),
    );

    tree
}

fn create_racer_end() -> DialogueTree {
    let npc = DialogueSpeaker::Npc(NPC_STREET_RACER);
    let mut tree = DialogueTree::new(209, "飆車族：認輸");

    tree.add_node(
        DialogueNode::new(0, npc, "靠！你真的贏了！我都不敢相信……那幾個彎你是怎麼過的！？")
            .with_emotion(SpeakerEmotion::Surprised)
            .then(1),
    );

    tree.add_node(
        DialogueNode::new(1, npc, "行，你夠厲害。這是賭金，一毛不少。")
            .with_emotion(SpeakerEmotion::Serious)
            .with_choice(
                DialogueChoice::simple("承讓了", 2)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_STREET_RACER,
                        delta: 5,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("下次再來一場！", 3)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_STREET_RACER,
                        delta: 10,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("想不想加入我？", 4)
                    .with_condition(DialogueCondition::RelationshipMin {
                        npc_id: NPC_STREET_RACER,
                        min: 15,
                    })
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_STREET_RACER,
                        delta: 20,
                    })
                    .with_consequence(DialogueConsequence::SetStoryFlag {
                        flag: "racer_ally".to_string(),
                        value: true,
                    }),
            ),
    );

    // 謙虛結束
    tree.add_node(
        DialogueNode::new(2, npc, "哼，下次我不會輸的。")
            .with_emotion(SpeakerEmotion::Serious)
            .with_choice(DialogueChoice::end("期待")),
    );

    // 再戰路線
    tree.add_node(
        DialogueNode::new(3, npc, "哈！你有種！好，下次我用我的改裝車來，看誰快！")
            .with_emotion(SpeakerEmotion::Smirk)
            .with_choice(
                DialogueChoice::end("隨時奉陪")
                    .with_consequence(DialogueConsequence::SetStoryFlag {
                        flag: "racer_rematch".to_string(),
                        value: true,
                    }),
            ),
    );

    // 招募路線（高好感度）
    tree.add_node(
        DialogueNode::new(4, npc, "加入你？……你是認真的？嗯……你開車確實比我強。好！以後你就是老大！")
            .with_emotion(SpeakerEmotion::Surprised)
            .with_choice(DialogueChoice::end("歡迎加入")),
    );

    tree
}

// ============================================================================
// #6 陰謀論部落客
// ============================================================================

fn create_blogger_start() -> DialogueTree {
    let npc = DialogueSpeaker::Npc(NPC_CONSPIRACY_BLOGGER);
    let mut tree = DialogueTree::new(210, "部落客：真相");

    tree.add_node(
        DialogueNode::new(0, npc, "噓！你！對，就是你！過來過來！")
            .with_emotion(SpeakerEmotion::Afraid)
            .then(1),
    );

    tree.add_node(
        DialogueNode::new(1, npc, "這座島有秘密！政府不想讓你知道的那種！我發現了三個可疑地點——廢棄工廠、山上電塔、港口貨櫃——都有異常電磁波！")
            .with_emotion(SpeakerEmotion::Surprised)
            .then(2),
    );

    tree.add_node(
        DialogueNode::new(2, npc, "我需要有人去現場拍照蒐證。你願意幫我揭發真相嗎！？")
            .with_emotion(SpeakerEmotion::Serious)
            .with_choice(
                DialogueChoice::simple("聽起來很有趣，我去查查", 3)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_CONSPIRACY_BLOGGER,
                        delta: 10,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("你是不是想太多了？", 4)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_CONSPIRACY_BLOGGER,
                        delta: -10,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("有報酬嗎？", 5),
            ),
    );

    // 好奇路線
    tree.add_node(
        DialogueNode::new(3, npc, "太好了！你是第一個相信我的人！去這三個地方看看，有什麼發現回來告訴我！")
            .with_emotion(SpeakerEmotion::Happy)
            .with_choice(DialogueChoice::end("我這就去")),
    );

    // 質疑路線
    tree.add_node(
        DialogueNode::new(4, npc, "想太多！？你看看我的鋁箔帽！這是防止他們讀取腦波的！你去看了就知道我說的是真的！")
            .with_emotion(SpeakerEmotion::Angry)
            .with_choice(
                DialogueChoice::simple("好吧，我去看看總行了吧", 3)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_CONSPIRACY_BLOGGER,
                        delta: 5,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("你需要看醫生", 6)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_CONSPIRACY_BLOGGER,
                        delta: -15,
                    }),
            ),
    );

    // 報酬路線
    tree.add_node(
        DialogueNode::new(5, npc, "報酬？真相就是最好的報酬！……好啦，我部落格有贊助商，事成之後分你一點。")
            .with_emotion(SpeakerEmotion::Thinking)
            .with_choice(
                DialogueChoice::simple("好，成交", 3)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_CONSPIRACY_BLOGGER,
                        delta: 5,
                    }),
            ),
    );

    // 嚴厲拒絕
    tree.add_node(
        DialogueNode::new(6, npc, "他們都這樣說！你也是他們的人！走開！")
            .with_emotion(SpeakerEmotion::Angry)
            .with_choice(DialogueChoice::end("告辭")),
    );

    tree
}

fn create_blogger_end() -> DialogueTree {
    let npc = DialogueSpeaker::Npc(NPC_CONSPIRACY_BLOGGER);
    let mut tree = DialogueTree::new(211, "部落客：發現");

    tree.add_node(
        DialogueNode::new(0, npc, "你回來了！讓我看看——天啊！這些照片！我就知道！")
            .with_emotion(SpeakerEmotion::Surprised)
            .then(1),
    );

    tree.add_node(
        DialogueNode::new(1, npc, "廢棄工廠裡有奇怪的設備、電塔發出不明訊號、港口的貨櫃都是空的卻有警衛！這絕對不正常！")
            .with_emotion(SpeakerEmotion::Serious)
            .with_choice(
                DialogueChoice::simple("確實有點可疑", 2)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_CONSPIRACY_BLOGGER,
                        delta: 10,
                    })
                    .with_consequence(DialogueConsequence::SetStoryFlag {
                        flag: "believes_conspiracy".to_string(),
                        value: true,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("可能只是巧合吧", 3)
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_CONSPIRACY_BLOGGER,
                        delta: -5,
                    }),
            )
            .with_choice(
                DialogueChoice::simple("我們一起繼續調查", 4)
                    .with_condition(DialogueCondition::RelationshipMin {
                        npc_id: NPC_CONSPIRACY_BLOGGER,
                        min: 15,
                    })
                    .with_consequence(DialogueConsequence::ChangeRelationship {
                        npc_id: NPC_CONSPIRACY_BLOGGER,
                        delta: 15,
                    })
                    .with_consequence(DialogueConsequence::SetStoryFlag {
                        flag: "conspiracy_partner".to_string(),
                        value: true,
                    }),
            ),
    );

    // 相信路線
    tree.add_node(
        DialogueNode::new(2, npc, "對吧對吧！我要把這些全部寫到部落格上！謝謝你的幫忙！這是你的報酬！")
            .with_emotion(SpeakerEmotion::Happy)
            .with_choice(DialogueChoice::end("小心一點")),
    );

    // 懷疑路線
    tree.add_node(
        DialogueNode::new(3, npc, "巧合？才不是！不過沒關係，我自己會繼續追查的。謝謝你的照片。")
            .with_emotion(SpeakerEmotion::Sad)
            .with_choice(DialogueChoice::end("隨你")),
    );

    // 合作路線（高好感度）
    tree.add_node(
        DialogueNode::new(4, npc, "真的嗎！？太好了！我一個人調查好孤獨……有你在我更有信心了！我們下次去挖更深的真相！")
            .with_emotion(SpeakerEmotion::Happy)
            .with_choice(DialogueChoice::end("好，隨時通知我")),
    );

    tree
}

// ============================================================================
// 測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_side_dialogues_registered() {
        let mut database = DialogueDatabase::default();
        register_side_dialogues(&mut database);

        // 12 個對話（6 個支線 × 2 start/end）
        for id in 200..=211 {
            assert!(
                database.get_dialogue(id).is_some(),
                "對話 ID {} 未註冊",
                id
            );
        }
    }

    #[test]
    fn test_all_side_npcs_registered() {
        let mut database = DialogueDatabase::default();
        register_side_dialogues(&mut database);

        // 6 個 NPC
        for id in 200..=205 {
            assert!(
                database.get_npc(id).is_some(),
                "NPC ID {} 未註冊",
                id
            );
        }
    }

    #[test]
    fn test_dialogues_have_start_nodes() {
        let mut database = DialogueDatabase::default();
        register_side_dialogues(&mut database);

        for id in 200..=211 {
            let tree = database.get_dialogue(id).unwrap();
            assert!(
                tree.get_node(tree.start_node).is_some(),
                "對話 {} 的起始節點 {} 不存在",
                id,
                tree.start_node
            );
        }
    }

    #[test]
    fn test_start_dialogues_have_choices() {
        let mut database = DialogueDatabase::default();
        register_side_dialogues(&mut database);

        // 每個開始對話（偶數 ID）至少有一個節點含分支選項
        for id in (200..=210).step_by(2) {
            let tree = database.get_dialogue(id).unwrap();
            let has_choices = tree.nodes.values().any(|node| !node.choices.is_empty());
            assert!(
                has_choices,
                "開始對話 {} 缺少分支選項",
                id
            );
        }
    }

    #[test]
    fn test_end_dialogues_have_relationship_choices() {
        let mut database = DialogueDatabase::default();
        register_side_dialogues(&mut database);

        // 每個結束對話（奇數 ID）至少有一個選項包含 ChangeRelationship
        for id in (201..=211).step_by(2) {
            let tree = database.get_dialogue(id).unwrap();
            let has_relationship_consequence = tree.nodes.values().any(|node| {
                node.choices.iter().any(|choice| {
                    choice.consequences.iter().any(|c| {
                        matches!(c, DialogueConsequence::ChangeRelationship { .. })
                    })
                })
            });
            assert!(
                has_relationship_consequence,
                "結束對話 {} 缺少好感度變化選項",
                id
            );
        }
    }

    #[test]
    fn test_end_dialogues_have_high_relationship_branch() {
        let mut database = DialogueDatabase::default();
        register_side_dialogues(&mut database);

        // 每個結束對話應有 RelationshipMin 條件的高好感度分支
        for id in (201..=211).step_by(2) {
            let tree = database.get_dialogue(id).unwrap();
            let has_relationship_gate = tree.nodes.values().any(|node| {
                node.choices.iter().any(|choice| {
                    matches!(
                        &choice.condition,
                        Some(DialogueCondition::RelationshipMin { .. })
                    )
                })
            });
            assert!(
                has_relationship_gate,
                "結束對話 {} 缺少高好感度專屬分支",
                id
            );
        }
    }

    #[test]
    fn test_dialogue_node_reachability() {
        let mut database = DialogueDatabase::default();
        register_side_dialogues(&mut database);

        // 驗證所有對話的起始節點可以到達至少一個結束點
        for id in 200..=211 {
            let tree = database.get_dialogue(id).unwrap();
            let mut visited = std::collections::HashSet::new();
            let mut stack = vec![tree.start_node];
            let mut found_end = false;

            while let Some(node_id) = stack.pop() {
                if !visited.insert(node_id) {
                    continue;
                }
                if let Some(node) = tree.get_node(node_id) {
                    if node.choices.is_empty() {
                        // 無選項：檢查 next_node 或已到結尾
                        if let Some(next) = node.next_node {
                            stack.push(next);
                        } else {
                            found_end = true;
                        }
                    } else {
                        for choice in &node.choices {
                            if choice.ends_dialogue {
                                found_end = true;
                            } else if let Some(next) = choice.next_node {
                                stack.push(next);
                            }
                        }
                    }
                }
            }

            assert!(
                found_end,
                "對話 {} 從起始節點無法到達任何結束點",
                id
            );
        }
    }
}
