use log::info;
use teloxide::{prelude::*, utils::command::BotCommands};

const GROQ_BASE_URL: &str = "https://api.groq.com/openai/v1";

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();

    let bot = Bot::from_env();
    bot.set_my_commands(Command::bot_commands()).await.unwrap();
    info!(
        "{} has started!",
        bot.get_me().send().await.unwrap().user.username.unwrap()
    );

    Command::repl(bot, answer).await;
}

#[derive(BotCommands, Clone, PartialEq)]
#[command(
    rename_rule = "lowercase",
    description = "The following commands are supported:"
)]
enum Command {
    #[command(description = "summarize the replied message")]
    Summarize,
    #[command(description = "explain the replied message in caveman language")]
    Caveman,
    #[command(description = "explain the replied message")]
    Explain,
    #[command(description = "help command")]
    Help,
    #[command(description = "summarize the last 100 messages")]
    SummarizeRecent,
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    let client = reqwest::Client::new();
    match cmd {
        Command::Summarize | Command::Caveman | Command::Explain => {
            let replied_msg = match msg.reply_to_message() {
                Some(msg) => msg,
                None => {
                    bot.send_message(msg.chat.id, "Reply to a message for this command.")
                        .await?;
                    return Ok(());
                }
            };

            if replied_msg.text().is_none() {
                bot.send_message(msg.chat.id, "The replied message is not a text message.")
                    .await?;
                return Ok(());
            }

            let response = client
                .post(format!("{}/chat/completions", GROQ_BASE_URL))
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", std::env::var("GROQ_API_KEY").unwrap()))
                .json(&serde_json::json!({
                    "model": "llama3-70b-8192",
                    "messages": [
                        {
                            "role": "system",
                            "content": match cmd {
                                Command::Caveman => "You are a caveman. Summarize the users message like a caveman would: all caps, many grammatical errors & similar.",
                                Command::Explain => "Explain the users message.",
                                Command::Summarize => "Summarize the user's message.",
                                _ => unreachable!()
                            }
                        },
                        {
                            "role": "user",
                            "content": replied_msg.text().unwrap_or_default()
                        }
                    ]
                }))
                .send()
                .await
                .unwrap();

            let status = response.status();

            let json = response.json::<serde_json::Value>().await.unwrap();

            let completion = json["choices"][0]["message"]["content"].as_str();
            if let Some(completion) = completion {
                bot.send_message(msg.chat.id, completion).await?
            } else {
                bot.send_message(
                    msg.chat.id,
                    format!("Failed to process the message: error {}", status),
                )
                .await?
            }
        }
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?
        }
        Command::SummarizeRecent => {
            bot.send_message(msg.chat.id, "This command is not implemented yet.")
                .await?
        }
    };

    Ok(())
}
