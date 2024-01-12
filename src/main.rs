mod lanraragi;

use lanraragi::args::args;
use lanraragi::archive::Archive;
use lanraragi::tag::{fetch_latest_cn_tag, parse_data};
use lanraragi::unhandle::add_and_save_no_handle;

use owo_colors::OwoColorize;
use tokio::task;

#[tokio::main]
async fn main() {
    args();
    println!("获取Lanraragi作品和最新的cn标签...");
    let fetch_task = task::spawn(Archive::fetch_archives());
    let tags_task = task::spawn(fetch_latest_cn_tag());

    // 等待两个任务完成
    let all_archive = fetch_task.await.unwrap();
    let tags = tags_task.await.unwrap();

    let tag_cn = parse_data(&tags).unwrap();
    let mut run_count = 0;

    println!("共有 {} 条作品", all_archive.len().bright_green());
    for archive in all_archive.iter() {
        run_count += 1;
        if !archive.is_empty_tags() {
            continue;
        }
        let gls = archive.search_from_eh().await;
        if gls.is_empty() {
            add_and_save_no_handle(archive.clone());
            continue;
        }
        let index = archive.print_and_get_index(&gls);
        if index > 0 && index <= gls.len() as i32 {
            let tags = &gls[index as usize - 1].get_tags_from_eh_gl(&tag_cn).await;
            archive
                .change_tags_to_lanraragi(&format!("{},{}", &archive.tags, &tags))
                .await;
            println!(
                "已处理 {}/{}  {:.1}%",
                run_count.green(),
                all_archive.len().cyan(),
                (run_count as f32 / all_archive.len() as f32 * 100.0).bright_green()
            );
        } else {
            add_and_save_no_handle(archive.clone());
            print!("❌not handle");
        }
    }
    println!("结束");
}
