//! 支線任務對話樹資料（第 4-6 組 NPC）
//!
//! 從 side_dialogues.rs 拆分，降低單檔行數。

use super::dialogue::*;

/// 支線任務 NPC ID（與 side_dialogues.rs 中的常數對應）
const NPC_STRAY_DOG_UNCLE: u32 = 203;
const NPC_STREET_RACER: u32 = 204;
const NPC_CONSPIRACY_BLOGGER: u32 = 205;

// ============================================================================
// #4 流浪狗大叔
// ============================================================================

pub(super) fn create_dog_uncle_start() -> DialogueTree {
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

pub(super) fn create_dog_uncle_end() -> DialogueTree {
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

pub(super) fn create_racer_start() -> DialogueTree {
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

pub(super) fn create_racer_end() -> DialogueTree {
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

pub(super) fn create_blogger_start() -> DialogueTree {
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

pub(super) fn create_blogger_end() -> DialogueTree {
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
