// ============================================================
// FILE:        PlanListScreen.kt
// MODULE:      Layer 4 — Android UI > Plan List Screen
// TASK:        T-045 (implementation — Session 4)
// PLATFORM:    android
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Phase 3, Android Compose UI
// DEPENDENCIES: Jetpack Compose (Material3), FocusMeDaemonService
// TEST COVERAGE: Test: plans list renders from service, Test: FAB navigates to create
// KNOWN LIMITATIONS: Uses singleton service access (FocusMeDaemonService.instance).
//                    Plan list refreshes on composition only — no LiveData/Flow yet.
// ============================================================

package com.focusme.android.ui.screens

import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.focusme.android.service.FocusMeDaemonService
import com.focusme.android.service.FocusMeDaemonService.PlanData

/**
 * PlanListScreen — displays all Focus Plans in a LazyColumn.
 *
 * Features:
 * - LazyColumn of plan cards (name, rule count, schedule summary)
 * - FAB to create a new plan (navigates to PlanEditScreen)
 * - Long-press on plan → context menu (Edit / Delete)
 * - Pull-to-refresh TBD (post-MVP)
 *
 * @param onCreatePlan   Navigate to PlanEditScreen in create mode
 * @param onEditPlan     Navigate to PlanEditScreen with planId
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalFoundationApi::class)
@Composable
fun PlanListScreen(
    onCreatePlan: () -> Unit,
    onEditPlan: (planId: String) -> Unit,
) {
    val daemon = FocusMeDaemonService.instance
    val plans = remember { mutableStateListOf<PlanData>() }

    // Load plans from service
    LaunchedEffect(daemon) {
        daemon?.let { svc ->
            plans.clear()
            plans.addAll(svc.getAllPlans())
        }
    }

    // Track which plan has the context menu open
    var contextMenuPlanId by remember { mutableStateOf<String?>(null) }
    var showDeleteDialog by remember { mutableStateOf(false) }
    var deletePlanId by remember { mutableStateOf("") }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Focus Plans") },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer,
                    titleContentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                ),
            )
        },
        floatingActionButton = {
            FloatingActionButton(
                onClick = onCreatePlan,
                containerColor = MaterialTheme.colorScheme.primary,
            ) {
                Icon(Icons.Filled.Add, contentDescription = "Create Plan")
            }
        },
    ) { padding ->
        if (daemon == null) {
            // Service not running
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    text = "FocusMe service is not running.\nPlease start the service first.",
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.error,
                )
            }
        } else if (plans.isEmpty()) {
            // No plans created yet
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding),
                contentAlignment = Alignment.Center,
            ) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Text(
                        text = "No Focus Plans yet",
                        style = MaterialTheme.typography.headlineSmall,
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(
                        text = "Tap + to create your first plan",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        } else {
            LazyColumn(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding),
                contentPadding = PaddingValues(16.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                items(plans, key = { it.id }) { plan ->
                    PlanCard(
                        plan = plan,
                        isContextMenuOpen = contextMenuPlanId == plan.id,
                        onTap = { onEditPlan(plan.id) },
                        onLongPress = { contextMenuPlanId = plan.id },
                        onDismissMenu = { contextMenuPlanId = null },
                        onEdit = {
                            contextMenuPlanId = null
                            onEditPlan(plan.id)
                        },
                        onDelete = {
                            contextMenuPlanId = null
                            deletePlanId = plan.id
                            showDeleteDialog = true
                        },
                    )
                }
            }
        }

        // Delete confirmation dialog
        if (showDeleteDialog) {
            AlertDialog(
                onDismissRequest = { showDeleteDialog = false },
                title = { Text("Delete Plan?") },
                text = { Text("This will permanently remove the Focus Plan and all its rules.") },
                confirmButton = {
                    TextButton(onClick = {
                        daemon?.removePlan(deletePlanId)
                        plans.removeAll { it.id == deletePlanId }
                        showDeleteDialog = false
                    }) {
                        Text("Delete", color = MaterialTheme.colorScheme.error)
                    }
                },
                dismissButton = {
                    TextButton(onClick = { showDeleteDialog = false }) {
                        Text("Cancel")
                    }
                },
            )
        }
    }
}

/**
 * Individual plan card with long-press context menu.
 */
@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun PlanCard(
    plan: PlanData,
    isContextMenuOpen: Boolean,
    onTap: () -> Unit,
    onLongPress: () -> Unit,
    onDismissMenu: () -> Unit,
    onEdit: () -> Unit,
    onDelete: () -> Unit,
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .combinedClickable(
                onClick = onTap,
                onLongClick = onLongPress,
            ),
        elevation = CardDefaults.cardElevation(defaultElevation = 2.dp),
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = plan.name,
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold,
                )
                if (plan.enabled) {
                    Badge(containerColor = MaterialTheme.colorScheme.primary) {
                        Text("Active", modifier = Modifier.padding(horizontal = 4.dp))
                    }
                }
            }

            Spacer(modifier = Modifier.height(8.dp))

            // Rule counts
            val appRuleCount = plan.rules.count { it.ruleType == "app" }
            val urlRuleCount = plan.rules.count { it.ruleType == "url" }
            Text(
                text = "$appRuleCount apps · $urlRuleCount URLs blocked",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            // Schedule summary
            val daysStr = formatScheduleDays(plan.scheduleDays)
            val timeStr = formatMinuteRange(plan.scheduleStartMin, plan.scheduleEndMin)
            Text(
                text = "$daysStr · $timeStr",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            // Context menu dropdown
            DropdownMenu(
                expanded = isContextMenuOpen,
                onDismissRequest = onDismissMenu,
            ) {
                DropdownMenuItem(
                    text = { Text("Edit") },
                    leadingIcon = { Icon(Icons.Filled.Edit, contentDescription = "Edit") },
                    onClick = onEdit,
                )
                DropdownMenuItem(
                    text = { Text("Delete", color = MaterialTheme.colorScheme.error) },
                    leadingIcon = {
                        Icon(
                            Icons.Filled.Delete,
                            contentDescription = "Delete",
                            tint = MaterialTheme.colorScheme.error,
                        )
                    },
                    onClick = onDelete,
                )
            }
        }
    }
}

// ---- Formatting Helpers ----

private fun formatScheduleDays(days: Set<Int>): String {
    if (days.size == 7) return "Every day"
    if (days == setOf(1, 2, 3, 4, 5)) return "Weekdays"
    if (days == setOf(6, 7)) return "Weekends"

    val dayNames = mapOf(
        1 to "Mon", 2 to "Tue", 3 to "Wed", 4 to "Thu",
        5 to "Fri", 6 to "Sat", 7 to "Sun",
    )
    return days.sorted().mapNotNull { dayNames[it] }.joinToString(", ")
}

private fun formatMinuteRange(startMin: Int, endMin: Int): String {
    return "${formatMinute(startMin)} – ${formatMinute(endMin)}"
}

private fun formatMinute(totalMinutes: Int): String {
    val h = totalMinutes / 60
    val m = totalMinutes % 60
    val period = if (h < 12) "AM" else "PM"
    val displayH = when {
        h == 0 -> 12
        h > 12 -> h - 12
        else -> h
    }
    return "$displayH:${m.toString().padStart(2, '0')} $period"
}
