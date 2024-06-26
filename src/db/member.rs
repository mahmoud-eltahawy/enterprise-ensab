use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use sqlx::{query, Pool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Default, PartialEq, Eq, Debug)]
pub struct RawMember {
    pub id: Uuid,
    pub name: String,
    pub is_male: bool,
    pub sons: Vec<RawMember>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SonlessRawMember {
    pub id: Uuid,
    pub name: String,
    pub is_male: bool,
}

#[cfg(feature = "ssr")]
pub async fn create(
    transaction: &mut Transaction<'_, Postgres>,
    RawMember {
        id,
        name,
        is_male,
        sons,
    }: RawMember,
    parent_id: Option<Uuid>,
) -> anyhow::Result<()> {
    use chrono::{NaiveDateTime, Utc};
    let now = Utc::now();
    let now = NaiveDateTime::new(now.date_naive(), now.time());
    query!(
        r#"
            INSERT INTO "member" (id,"name",is_male,insert_date,parent_id)
            VALUES($1,$2,$3,$4,$5);
            "#,
        id,
        name,
        is_male,
        now,
        parent_id
    )
    .execute(&mut **transaction)
    .await?;
    for son in sons {
        Box::pin(create(transaction, son, Some(id))).await?;
    }

    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn update(
    transaction: &mut Transaction<'_, Postgres>,
    members: Vec<SonlessRawMember>,
) -> anyhow::Result<()> {
    for member in members {
        query!(
            r#"
                update member set name = $2,is_male = $3 where id = $1
                "#,
            member.id,
            member.name,
            member.is_male
        )
        .execute(&mut **transaction)
        .await?;
    }
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn delete(transaction: &mut Transaction<'_, Postgres>, id: Uuid) -> anyhow::Result<()> {
    query!("delete from member where id = $1", id)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn read(pool: &Pool<Postgres>, id: Uuid) -> anyhow::Result<RawMember> {
    let sons_ids = query!(
        "select id from member where parent_id = $1 and id <> uuid_nil()",
        id
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|x| x.id)
    .collect::<Vec<_>>();
    let mut sons = Vec::new();
    for son_id in sons_ids {
        let son = Box::pin(read(pool, son_id)).await?;
        sons.push(son);
    }
    let member_record = query!("select name,is_male from member where id = $1", id)
        .fetch_one(pool)
        .await?;
    let name: String = member_record.name;
    let is_male: bool = member_record.is_male;
    Ok(RawMember {
        id,
        name,
        is_male,
        sons,
    })
}
