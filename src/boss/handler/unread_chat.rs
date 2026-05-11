use std::time::Duration;

use anyhow::{Context, anyhow};
use rust_drission::utils::sleep_random_ms;

use crate::{
    boss::{
        BOSS_CHAT_URL,
        model::{ChatMessage, UnreadChat},
    },
    browser,
};

// 消息通知监听 — 返回"未读"标签下的所有聊天条目
pub fn get_unread_chat() -> Result<Vec<UnreadChat>, anyhow::Error> {
    let unread_chat_data = browser::with_boss_tab(|page| {
        page.get(BOSS_CHAT_URL)?;
        sleep_random_ms(1200, 1500);
        click_label(page, "全部")?;
        sleep_random_ms(1200, 1500);
        let js_result = page.run_js(EXTRACT_JS)?;
        parse_unread_chats(&js_result)
    })?;

    Ok(unread_chat_data)
}

// 从 JS 执行结果中解析聊天列表
fn parse_unread_chats(js_result: &serde_json::Value) -> Result<Vec<UnreadChat>, anyhow::Error> {
    let json_str = js_result
        .as_str()
        .or_else(|| js_result.get("value").and_then(|v| v.as_str()))
        .context("JS 返回值不是字符串")?;

    serde_json::from_str(json_str).context("未读聊天 JSON 解析失败")
}

// 点击不同 label 的聊天
fn click_label(page: &rust_drission::Page, name: &str) -> Result<(), anyhow::Error> {
    let label_name_eles = page.eles(".label-list .label-name")?;

    for label_name_ele in label_name_eles {
        let label_content = label_name_ele.text_content()?.trim().to_string();
        if label_content == name || label_content.starts_with(name) {
            label_name_ele.click()?;
            break;
        }
    }
    Ok(())
}

// 提取每个 <li> 中的聊天信息，返回 JSON 字符串
//
// DOM 结构（参见 unread_chat_dom.html）:
//   .user-list-content li
//     .friend-content              — 聊天项容器
//       .figure
//         .notice-badge            — 未读数量
//         img.image-circle         — 头像 URL (src)
//       .text
//         .time                    — 时间
//         .name-box
//           .name-text             — 招聘者姓名
//           span (第1个非.name-text) — 公司名称
//           span (第2个非.name-text) — 职位/头衔（在 .vline 之后）
//         .last-msg-text           — 最近消息内容
const EXTRACT_JS: &str = r#"
(() => {
    const normalizeText = (value) => value ? value.replace(/\s+/g, ' ').trim() : '';

    const items = document.querySelectorAll('.friend-content');
    const results = [];

    for (const li of items) {
        const badgeEl   = li.querySelector('.notice-badge');
        const avatarEl  = li.querySelector('.figure img.image-circle');
        const timeEl    = li.querySelector('.time');
        const nameEl    = li.querySelector('.name-text');
        const lastMsgEl = li.querySelector('.last-msg-text');

        // .name-box 包含: .name-text、公司 span、.vline、职位 span
        const nameBox = li.querySelector('.name-box');
        let company = '';
        let title   = '';
        if (nameBox) {
            // 取 .name-box 下所有直接子 span（排除 .name-text）
            const spans = Array.from(nameBox.querySelectorAll('span:not(.name-text)'));
            if (spans.length >= 1) company = normalizeText(spans[0].textContent);
            if (spans.length >= 2) title   = normalizeText(spans[1].textContent);
        }

        const unreadCount = badgeEl ? parseInt(badgeEl.textContent.trim(), 10) || 0 : 0;

        results.push({
            idx:          results.length,
            name:         nameEl    ? normalizeText(nameEl.textContent)    : '',
            company,
            title,
            unread_count: unreadCount,
            time:         timeEl    ? normalizeText(timeEl.textContent)    : '',
            last_message: lastMsgEl ? normalizeText(lastMsgEl.textContent) : '',
            avatar_url:   avatarEl  ? (avatarEl.src || '')                 : '',
        });
    }

    return JSON.stringify(results);
})()
"#;

// 获取未读的聊天消息详情 根据idx 去点击 监听接口响应数据 获取 消息列表
// 聊天消息接口：https://www.zhipin.com/wapi/zpchat/geek/historyMsg
// 响应数据参考：src/boss/handler/chat_history_resp.json
pub fn get_unread_chat_message(idx: usize) -> Result<Vec<ChatMessage>, anyhow::Error> {
    browser::with_boss_tab(|page| {
        // page.get(BOSS_CHAT_URL)?;
        // sleep_random_ms(700, 1000);
        // click_label(page, "未读")?;
        // sleep_random_ms(1000, 1200);

        // 在导航/点击前先设置网络监听，避免错过响应事件
        let listener = page.listen_url("zpchat/geek/historyMsg")?;

        // 找到第 idx 个聊天条目并点击
        let items = page.eles(".friend-content")?;
        let item = items
            .into_iter()
            .nth(idx)
            .ok_or_else(|| anyhow!("未找到第 {idx} 个聊天条目"))?;
        item.click()?;

        // 等待接口响应（最多 10 秒）
        let packet = listener
            .wait(Duration::from_secs(10))?
            .ok_or_else(|| anyhow!("等待 historyMsg 接口响应超时"))?;

        let body_bytes = packet
            .body
            .ok_or_else(|| anyhow!("historyMsg 响应 body 为空"))?;
        let body_str = String::from_utf8(body_bytes).context("historyMsg 响应非 UTF-8 编码")?;

        parse_chat_messages(&body_str)
    })
}

// 从 historyMsg 接口响应中解析消息列表
//
// 发送方判断逻辑：
//   第一条 body.type == 8 的消息携带 body.jobDesc.boss.uid（招聘者 uid）。
//   后续每条消息：from.uid == boss_uid 则 received=true（招聘者发来），否则 received=false（自己发送）。
fn parse_chat_messages(body: &str) -> Result<Vec<ChatMessage>, anyhow::Error> {
    let root: serde_json::Value = serde_json::from_str(body).context("historyMsg JSON 解析失败")?;

    let code = root.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
    if code != 0 {
        return Err(anyhow!(
            "historyMsg 接口业务错误: code={}, message={}",
            code,
            root.get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
        ));
    }

    let messages = root
        .pointer("/zpData/messages")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("historyMsg 响应缺少 zpData.messages 数组"))?;

    // 从第一条 body.type == 8 的消息中取出 boss uid
    let boss_uid: Option<i64> = messages.iter().find_map(|msg| {
        let body_type = msg.pointer("/body/type").and_then(|v| v.as_i64())?;
        if body_type == 8 {
            msg.pointer("/body/jobDesc/boss/uid")
                .and_then(|v| v.as_i64())
        } else {
            None
        }
    });

    let result = messages
        .iter()
        .filter_map(|msg| {
            // 只保留 body.type == 1 的普通文本消息，其余消息（系统卡片等）过滤掉
            let body_type = msg.pointer("/body/type").and_then(|v| v.as_i64())?;
            if body_type != 1 {
                return None;
            }
            let text = msg
                .pointer("/body/text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let mid = msg.get("mid").and_then(|v| v.as_i64())?;
            let from_uid = msg.pointer("/from/uid").and_then(|v| v.as_i64());
            // received=true 表示招聘者（boss）发来的消息，false 表示自己发送的
            let received = match (boss_uid, from_uid) {
                (Some(boss), Some(from)) => from == boss,
                _ => true, // 无法判断时默认视为对方发来
            };
            let time = msg.get("time").and_then(|v| v.as_i64()).unwrap_or(0);
            let from_name = msg
                .pointer("/from/name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            Some(ChatMessage {
                mid,
                received,
                text,
                time,
                from_name,
            })
        })
        .collect();

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::{parse_chat_messages, parse_unread_chats};
    use serde_json::json;

    #[test]
    fn parses_unread_chat_from_js_string_result() {
        let js_result = json!(
            r#"[{"idx":0,"name":"宋昊","company":"科锐国际","title":"猎头顾问","unread_count":2,"time":"19:11","last_message":"您正在与Boss宋昊沟通","avatar_url":"https://img.bosszhipin.com/example.jpg"}]"#
        );

        let chats = parse_unread_chats(&js_result).expect("should parse chats");

        assert_eq!(chats.len(), 1);
        assert_eq!(chats[0].idx, 0);
        assert_eq!(chats[0].name, "宋昊");
        assert_eq!(chats[0].company, "科锐国际");
        assert_eq!(chats[0].title, "猎头顾问");
        assert_eq!(chats[0].unread_count, 2);
        assert_eq!(chats[0].time, "19:11");
        assert_eq!(chats[0].last_message, "您正在与Boss宋昊沟通");
        assert_eq!(
            chats[0].avatar_url,
            "https://img.bosszhipin.com/example.jpg"
        );
    }

    #[test]
    fn parses_unread_chat_from_wrapped_js_result() {
        let js_result = json!({
            "value": r#"[{"idx":0,"name":"卢女士","company":"外企德科数字","title":"招聘经理","unread_count":1,"time":"16:17","last_message":"Hello，你好啊","avatar_url":"https://img.bosszhipin.com/example2.jpg"}]"#
        });

        let chats = parse_unread_chats(&js_result).expect("should parse wrapped chats");

        assert_eq!(chats.len(), 1);
        assert_eq!(chats[0].idx, 0);
        assert_eq!(chats[0].name, "卢女士");
        assert_eq!(chats[0].unread_count, 1);
        assert_eq!(chats[0].time, "16:17");
    }

    #[test]
    fn returns_empty_vec_for_empty_array() {
        let js_result = json!("[]");
        let chats = parse_unread_chats(&js_result).expect("should parse empty array");
        assert!(chats.is_empty());
    }

    #[test]
    fn parses_text_messages_from_history_response() {
        // boss uid = 100，geek uid = 200
        // 第一条消息（body.type==8）携带 jobDesc.boss.uid=100，用于后续判断发送方
        // 第二条消息 from.uid=100（boss 发来）→ received=true
        // 第三条消息 from.uid=200（自己发送）→ received=false
        // 第四条消息 body.type==8 系统卡片，被过滤
        let body = r#"{
            "code": 0,
            "message": "Success",
            "zpData": {
                "hasMore": false,
                "messages": [
                    {
                        "mid": 1,
                        "received": true,
                        "type": 3,
                        "body": {
                            "type": 8,
                            "templateId": 1,
                            "headTitle": "Boss希望与您沟通",
                            "jobDesc": { "boss": { "uid": 100, "name": "孙坤宇" } }
                        },
                        "from": { "uid": 100, "name": "孙坤宇" },
                        "to": { "uid": 200, "name": "张德宁" },
                        "time": 1778221353221
                    },
                    {
                        "mid": 111,
                        "received": true,
                        "type": 3,
                        "body": { "type": 1, "text": "华为开发岗考虑吗", "templateId": 1, "headTitle": "" },
                        "from": { "uid": 100, "name": "孙坤宇" },
                        "to": { "uid": 200, "name": "张德宁" },
                        "time": 1778221375022
                    },
                    {
                        "mid": 222,
                        "received": true,
                        "type": 1,
                        "body": { "type": 1, "text": "不考虑", "templateId": 1, "headTitle": "" },
                        "from": { "uid": 200, "name": "张德宁" },
                        "to": { "uid": 100, "name": "孙坤宇" },
                        "time": 1778254954016
                    }
                ],
                "type": 1,
                "minMsgId": 1
            }
        }"#;

        let msgs = parse_chat_messages(body).expect("should parse messages");

        // body.type==8 的卡片消息被过滤，只保留两条文本消息
        assert_eq!(msgs.len(), 2);

        // mid=111：from.uid(100) == boss_uid(100) → received=true（boss 发来）
        assert_eq!(msgs[0].mid, 111);
        assert!(msgs[0].received);
        assert_eq!(msgs[0].text, "华为开发岗考虑吗");
        assert_eq!(msgs[0].from_name, "孙坤宇");
        assert_eq!(msgs[0].time, 1778221375022);

        // mid=222：from.uid(200) != boss_uid(100) → received=false（自己发送）
        assert_eq!(msgs[1].mid, 222);
        assert!(!msgs[1].received);
        assert_eq!(msgs[1].text, "不考虑");
        assert_eq!(msgs[1].from_name, "张德宁");
    }

    #[test]
    fn returns_error_for_non_zero_code() {
        let body = r#"{"code": 1, "message": "未登录"}"#;
        let err = parse_chat_messages(body).expect_err("non-zero code should fail");
        assert!(err.to_string().contains("historyMsg 接口业务错误"));
    }
}
