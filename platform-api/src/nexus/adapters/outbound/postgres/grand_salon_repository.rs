use super::pg_err;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use platform_core::nexus::{
    domain::{
        entities::grand_salon::{
            Cercle, CercleKind, Dossier, GazetteArticle, Habitué, MotionDuSalon, MotionStatus,
            Ressources,
        },
        errors::DomainError,
    },
    ports::outbound::grand_salon_repository::GrandSalonRepository,
};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgGrandSalonRepository {
    pool: PgPool,
}
impl PgGrandSalonRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn status(value: &str) -> Result<MotionStatus, DomainError> {
    match value {
        "en_vote" => Ok(MotionStatus::EnVote),
        "adoptee" => Ok(MotionStatus::Adoptee),
        "rejetee" => Ok(MotionStatus::Rejetee),
        _ => Err(DomainError::Infrastructure(
            "statut Grand Salon invalide".into(),
        )),
    }
}

fn cercle_kind(value: &str) -> Result<CercleKind, DomainError> {
    match value {
        "bande" => Ok(CercleKind::Bande),
        "club" => Ok(CercleKind::Club),
        "collectif" => Ok(CercleKind::Collectif),
        _ => Err(DomainError::Infrastructure(
            "type de cercle invalide".into(),
        )),
    }
}

#[async_trait]
impl GrandSalonRepository for PgGrandSalonRepository {
    async fn find_habitue(&self, g: &str, u: &str) -> Result<Option<Habitué>, DomainError> {
        let r:Option<(Uuid,String,String,String,i64,i64,i64,i64,i64,DateTime<Utc>)>=sqlx::query_as("SELECT id,guild_id,user_id,display_name,rayonnement,jetons,reputation,bons_plans,reseau,joined_at FROM grand_salon_habitues WHERE guild_id=$1 AND user_id=$2").bind(g).bind(u).fetch_optional(&self.pool).await.map_err(pg_err)?;
        Ok(r.map(
            |(
                id,
                guild_id,
                user_id,
                display_name,
                rayonnement,
                jetons,
                reputation,
                bons_plans,
                reseau,
                joined_at,
            )| Habitué {
                id,
                guild_id,
                user_id,
                display_name,
                ressources: Ressources {
                    rayonnement,
                    jetons,
                    reputation,
                    bons_plans,
                    reseau,
                },
                joined_at,
            },
        ))
    }
    async fn save_habitue(&self, h: &Habitué) -> Result<(), DomainError> {
        sqlx::query("INSERT INTO grand_salon_habitues (id,guild_id,user_id,display_name,rayonnement,jetons,reputation,bons_plans,reseau,joined_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)").bind(h.id).bind(&h.guild_id).bind(&h.user_id).bind(&h.display_name).bind(h.ressources.rayonnement).bind(h.ressources.jetons).bind(h.ressources.reputation).bind(h.ressources.bons_plans).bind(h.ressources.reseau).bind(h.joined_at).execute(&self.pool).await.map_err(pg_err)?;
        Ok(())
    }
    async fn claim_daily(&self, habitue_id: Uuid) -> Result<bool, DomainError> {
        let changed=sqlx::query("WITH claimed AS (INSERT INTO grand_salon_daily_claims (habitue_id) VALUES ($1) ON CONFLICT DO NOTHING RETURNING habitue_id) UPDATE grand_salon_habitues h SET rayonnement=h.rayonnement+10,jetons=h.jetons+50,reputation=h.reputation+2,bons_plans=h.bons_plans+3,reseau=h.reseau+2 FROM claimed WHERE h.id=claimed.habitue_id").bind(habitue_id).execute(&self.pool).await.map_err(pg_err)?.rows_affected();
        Ok(changed == 1)
    }
    async fn create_cercle(&self, c: &Cercle) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        let kind = match c.kind {
            CercleKind::Bande => "bande",
            CercleKind::Club => "club",
            CercleKind::Collectif => "collectif",
        };
        sqlx::query("INSERT INTO grand_salon_cercles (id,guild_id,kind,name,devise,caisse,reputation,rayonnement,founder_id,created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)")
            .bind(c.id).bind(&c.guild_id).bind(kind).bind(&c.name).bind(&c.devise).bind(c.caisse).bind(c.reputation).bind(c.rayonnement).bind(c.founder_id).bind(c.created_at).execute(&mut *tx).await.map_err(pg_err)?;
        sqlx::query("INSERT INTO grand_salon_cercle_members (cercle_id,habitue_id,role) VALUES ($1,$2,'fondateur')").bind(c.id).bind(c.founder_id).execute(&mut *tx).await.map_err(pg_err)?;
        tx.commit().await.map_err(pg_err)?;
        Ok(())
    }
    async fn list_cercles(&self, guild_id: &str) -> Result<Vec<Cercle>, DomainError> {
        let rows:Vec<(Uuid,String,String,String,String,i64,i64,i64,Uuid,DateTime<Utc>,Option<DateTime<Utc>>)>=sqlx::query_as("SELECT id,guild_id,kind,name,devise,caisse,reputation,rayonnement,founder_id,created_at,dissolved_at FROM grand_salon_cercles WHERE guild_id=$1 AND dissolved_at IS NULL ORDER BY rayonnement DESC,name").bind(guild_id).fetch_all(&self.pool).await.map_err(pg_err)?;
        rows.into_iter()
            .map(
                |(
                    id,
                    guild_id,
                    k,
                    name,
                    devise,
                    caisse,
                    reputation,
                    rayonnement,
                    founder_id,
                    created_at,
                    dissolved_at,
                )| {
                    Ok(Cercle {
                        id,
                        guild_id,
                        kind: cercle_kind(&k)?,
                        name,
                        devise,
                        caisse,
                        reputation,
                        rayonnement,
                        founder_id,
                        created_at,
                        dissolved_at,
                    })
                },
            )
            .collect()
    }
    async fn create_motion(&self, m: &MotionDuSalon) -> Result<(), DomainError> {
        sqlx::query("INSERT INTO grand_salon_motions (id,guild_id,titre,texte,status,author_id,closes_at,soutien_pour,soutien_contre) VALUES ($1,$2,$3,$4,'en_vote',$5,$6,$7,$8)").bind(m.id).bind(&m.guild_id).bind(&m.titre).bind(&m.texte).bind(m.author_id).bind(m.closes_at).bind(m.soutien_pour).bind(m.soutien_contre).execute(&self.pool).await.map_err(pg_err)?;
        Ok(())
    }
    async fn list_motions(&self, guild_id: &str) -> Result<Vec<MotionDuSalon>, DomainError> {
        let rows: Vec<(Uuid, String, String, String, String, Uuid, DateTime<Utc>, i64, i64)> = sqlx::query_as(
            "SELECT id,guild_id,titre,texte,status,author_id,closes_at,soutien_pour,soutien_contre FROM grand_salon_motions WHERE guild_id=$1 ORDER BY closes_at DESC LIMIT 100"
        ).bind(guild_id).fetch_all(&self.pool).await.map_err(pg_err)?;
        rows.into_iter()
            .map(
                |(
                    id,
                    guild_id,
                    titre,
                    texte,
                    s,
                    author_id,
                    closes_at,
                    soutien_pour,
                    soutien_contre,
                )| {
                    Ok(MotionDuSalon {
                        id,
                        guild_id,
                        titre,
                        texte,
                        status: status(&s)?,
                        author_id,
                        closes_at,
                        soutien_pour,
                        soutien_contre,
                    })
                },
            )
            .collect()
    }
    async fn cast_vote(
        &self,
        motion_id: Uuid,
        habitue_id: Uuid,
        choice: bool,
        weight: i64,
    ) -> Result<(), DomainError> {
        sqlx::query("INSERT INTO grand_salon_votes (motion_id,habitue_id,choice,weight) VALUES ($1,$2,$3,$4) ON CONFLICT (motion_id,habitue_id) DO UPDATE SET choice=EXCLUDED.choice,weight=EXCLUDED.weight,created_at=NOW()")
            .bind(motion_id).bind(habitue_id).bind(choice).bind(weight).execute(&self.pool).await.map_err(pg_err)?;
        Ok(())
    }
    async fn vote_totals(&self, motion_id: Uuid) -> Result<(i64, i64), DomainError> {
        let (pour, contre): (i64, i64) = sqlx::query_as("SELECT COALESCE(SUM(weight) FILTER (WHERE choice),0)::BIGINT, COALESCE(SUM(weight) FILTER (WHERE NOT choice),0)::BIGINT FROM grand_salon_votes WHERE motion_id=$1")
            .bind(motion_id).fetch_one(&self.pool).await.map_err(pg_err)?;
        Ok((pour, contre))
    }
    async fn due_motions(&self) -> Result<Vec<MotionDuSalon>, DomainError> {
        let rows:Vec<(Uuid,String,String,String,String,Uuid,DateTime<Utc>,i64,i64)>=sqlx::query_as("SELECT id,guild_id,titre,texte,status,author_id,closes_at,soutien_pour,soutien_contre FROM grand_salon_motions WHERE status='en_vote' AND closes_at<=NOW()").fetch_all(&self.pool).await.map_err(pg_err)?;
        rows.into_iter()
            .map(
                |(
                    id,
                    guild_id,
                    titre,
                    texte,
                    s,
                    author_id,
                    closes_at,
                    soutien_pour,
                    soutien_contre,
                )| {
                    Ok(MotionDuSalon {
                        id,
                        guild_id,
                        titre,
                        texte,
                        status: status(&s)?,
                        author_id,
                        closes_at,
                        soutien_pour,
                        soutien_contre,
                    })
                },
            )
            .collect()
    }
    async fn close_motion(&self, id: Uuid, adopted: bool) -> Result<(), DomainError> {
        sqlx::query("UPDATE grand_salon_motions SET status=$2,closed_at=NOW() WHERE id=$1 AND status='en_vote'").bind(id).bind(if adopted{"adoptee"}else{"rejetee"}).execute(&self.pool).await.map_err(pg_err)?;
        Ok(())
    }
    async fn publish_gazette(&self, a: &GazetteArticle) -> Result<(), DomainError> {
        sqlx::query("INSERT INTO grand_salon_gazette (id,guild_id,headline,body,published_at) VALUES ($1,$2,$3,$4,$5)").bind(a.id).bind(&a.guild_id).bind(&a.headline).bind(&a.body).bind(a.published_at).execute(&self.pool).await.map_err(pg_err)?;
        Ok(())
    }
    async fn list_gazette(&self, guild_id: &str) -> Result<Vec<GazetteArticle>, DomainError> {
        let rows: Vec<(Uuid, String, String, String, DateTime<Utc>)> = sqlx::query_as("SELECT id,guild_id,headline,body,published_at FROM grand_salon_gazette WHERE guild_id=$1 ORDER BY published_at DESC LIMIT 50")
            .bind(guild_id).fetch_all(&self.pool).await.map_err(pg_err)?;
        Ok(rows
            .into_iter()
            .map(
                |(id, guild_id, headline, body, published_at)| GazetteArticle {
                    id,
                    guild_id,
                    headline,
                    body,
                    published_at,
                },
            )
            .collect())
    }
    async fn create_dossier(&self, d: &Dossier) -> Result<(), DomainError> {
        sqlx::query("INSERT INTO grand_salon_dossiers (id,guild_id,owner_id,subject,verified,revealed_at) VALUES ($1,$2,$3,$4,$5,$6)").bind(d.id).bind(&d.guild_id).bind(d.owner_id).bind(&d.subject).bind(d.verified).bind(d.revealed_at).execute(&self.pool).await.map_err(pg_err)?;
        Ok(())
    }
    async fn list_dossiers(
        &self,
        guild_id: &str,
        owner_id: Uuid,
    ) -> Result<Vec<Dossier>, DomainError> {
        let rows:Vec<(Uuid,String,Uuid,String,bool,Option<DateTime<Utc>>)>=sqlx::query_as("SELECT id,guild_id,owner_id,subject,verified,revealed_at FROM grand_salon_dossiers WHERE guild_id=$1 AND owner_id=$2 ORDER BY revealed_at NULLS FIRST").bind(guild_id).bind(owner_id).fetch_all(&self.pool).await.map_err(pg_err)?;
        Ok(rows
            .into_iter()
            .map(
                |(id, guild_id, owner_id, subject, verified, revealed_at)| Dossier {
                    id,
                    guild_id,
                    owner_id,
                    subject,
                    verified,
                    revealed_at,
                },
            )
            .collect())
    }
    async fn reveal_dossier(&self, dossier_id: Uuid, owner_id: Uuid) -> Result<(), DomainError> {
        let changed=sqlx::query("UPDATE grand_salon_dossiers SET revealed_at=NOW() WHERE id=$1 AND owner_id=$2 AND verified AND revealed_at IS NULL").bind(dossier_id).bind(owner_id).execute(&self.pool).await.map_err(pg_err)?.rows_affected();
        if changed == 0 {
            return Err(DomainError::ValidationError(
                "dossier deja revele ou non verifie".into(),
            ));
        }
        Ok(())
    }
}
