using Microsoft.EntityFrameworkCore;

namespace Orchestration.Infrastructure.Data;

/// <summary>
/// Entity Framework DbContext for the Orchestration system.
/// </summary>
public class OrchestrationDbContext : DbContext
{
    public OrchestrationDbContext(DbContextOptions<OrchestrationDbContext> options)
        : base(options)
    {
    }

    public DbSet<OnboardingRecord> OnboardingRecords => Set<OnboardingRecord>();
    public DbSet<IdempotencyRecord> IdempotencyRecords => Set<IdempotencyRecord>();
    public DbSet<WorkflowAuditRecord> WorkflowAuditRecords => Set<WorkflowAuditRecord>();

    protected override void OnModelCreating(ModelBuilder modelBuilder)
    {
        base.OnModelCreating(modelBuilder);

        modelBuilder.Entity<OnboardingRecord>(entity =>
        {
            entity.HasKey(e => e.Id);
            entity.HasIndex(e => e.EntityId);
            entity.HasIndex(e => e.IdempotencyKey).IsUnique();
            entity.Property(e => e.Status).HasMaxLength(50);
            entity.Property(e => e.EntityId).HasMaxLength(100);
            entity.Property(e => e.IdempotencyKey).HasMaxLength(200);
        });

        modelBuilder.Entity<IdempotencyRecord>(entity =>
        {
            entity.HasKey(e => e.Key);
            entity.Property(e => e.Key).HasMaxLength(500);
            entity.HasIndex(e => e.ExpiresAt);
        });

        modelBuilder.Entity<WorkflowAuditRecord>(entity =>
        {
            entity.HasKey(e => e.Id);
            entity.HasIndex(e => e.InstanceId);
            entity.HasIndex(e => e.EntityId);
            entity.HasIndex(e => e.Timestamp);
            entity.Property(e => e.EventType).HasMaxLength(100);
        });
    }
}

/// <summary>
/// Record for onboarding entities.
/// </summary>
public class OnboardingRecord
{
    public string Id { get; set; } = Guid.NewGuid().ToString();
    public required string EntityId { get; set; }
    public string Status { get; set; } = "Created";
    public string? IdempotencyKey { get; set; }
    public string? DataJson { get; set; }
    public DateTimeOffset CreatedAt { get; set; } = DateTimeOffset.UtcNow;
    public DateTimeOffset? UpdatedAt { get; set; }
    public string? WorkflowInstanceId { get; set; }
}

/// <summary>
/// Record for idempotency tracking.
/// </summary>
public class IdempotencyRecord
{
    public required string Key { get; set; }
    public required string ResultJson { get; set; }
    public DateTimeOffset CreatedAt { get; set; } = DateTimeOffset.UtcNow;
    public DateTimeOffset ExpiresAt { get; set; } = DateTimeOffset.UtcNow.AddDays(7);
}

/// <summary>
/// Audit record for workflow events.
/// </summary>
public class WorkflowAuditRecord
{
    public string Id { get; set; } = Guid.NewGuid().ToString();
    public required string InstanceId { get; set; }
    public required string EntityId { get; set; }
    public required string EventType { get; set; }
    public string? StepName { get; set; }
    public string? DataJson { get; set; }
    public DateTimeOffset Timestamp { get; set; } = DateTimeOffset.UtcNow;
}
