// ============================================================
// FILE:        MainNavigation.kt
// MODULE:      Layer 4 — Android UI > Navigation Host
// TASK:        T-045 (implementation — Session 4)
// PLATFORM:    android
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Phase 3, Android Compose UI
// DEPENDENCIES: Jetpack Compose (Material3), Navigation Compose
// TEST COVERAGE: Test: bottom nav switches between screens,
//                Test: plan creation navigates to edit and back
// KNOWN LIMITATIONS: Uses string routes (not type-safe navigation).
//                    Deep link support for plan editing TBD (post-MVP).
// ============================================================

package com.focusme.android.ui.screens

import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.List
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.navigation.NavHostController
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument

/**
 * MainNavigation — top-level composable that wires all screens
 * with a NavHost and bottom navigation bar.
 *
 * Navigation graph:
 *   plans/list          → PlanListScreen
 *   plans/edit?id={id}  → PlanEditScreen (edit existing plan)
 *   plans/create        → PlanEditScreen (create new plan)
 *   settings            → SettingsScreen
 *
 * Bottom bar items: Plans | Settings
 */
@Composable
fun MainNavigation() {
    val navController = rememberNavController()

    Scaffold(
        bottomBar = {
            FocusMeBottomBar(navController = navController)
        },
    ) { innerPadding ->
        NavHost(
            navController = navController,
            startDestination = Screen.PlanList.route,
            modifier = Modifier.padding(innerPadding),
        ) {
            // ---- Plan List ----
            composable(Screen.PlanList.route) {
                PlanListScreen(
                    onCreatePlan = {
                        navController.navigate(Screen.PlanCreate.route)
                    },
                    onEditPlan = { planId ->
                        navController.navigate("plans/edit?id=$planId")
                    },
                )
            }

            // ---- Plan Create (new plan) ----
            composable(Screen.PlanCreate.route) {
                PlanEditScreen(
                    planId = null,
                    onNavigateBack = { navController.popBackStack() },
                )
            }

            // ---- Plan Edit (existing plan) ----
            composable(
                route = "plans/edit?id={planId}",
                arguments = listOf(
                    navArgument("planId") {
                        type = NavType.StringType
                        nullable = true
                        defaultValue = null
                    },
                ),
            ) { backStackEntry ->
                val planId = backStackEntry.arguments?.getString("planId")
                PlanEditScreen(
                    planId = planId,
                    onNavigateBack = { navController.popBackStack() },
                )
            }

            // ---- Settings ----
            composable(Screen.Settings.route) {
                SettingsScreen(
                    onNavigateBack = { navController.popBackStack() },
                )
            }
        }
    }
}

// ============ Bottom Navigation ============

/**
 * Bottom navigation bar with Plans and Settings tabs.
 */
@Composable
private fun FocusMeBottomBar(navController: NavHostController) {
    val items = listOf(
        BottomNavItem(Screen.PlanList, "Plans", Icons.Filled.List),
        BottomNavItem(Screen.Settings, "Settings", Icons.Filled.Settings),
    )

    val navBackStackEntry by navController.currentBackStackEntryAsState()
    val currentRoute = navBackStackEntry?.destination?.route

    NavigationBar {
        items.forEach { item ->
            NavigationBarItem(
                selected = currentRoute == item.screen.route ||
                    (item.screen == Screen.PlanList && currentRoute?.startsWith("plans") == true),
                onClick = {
                    navController.navigate(item.screen.route) {
                        // Pop up to the start destination to avoid building a large back stack
                        popUpTo(Screen.PlanList.route) {
                            saveState = true
                        }
                        launchSingleTop = true
                        restoreState = true
                    }
                },
                icon = {
                    Icon(item.icon, contentDescription = item.label)
                },
                label = { Text(item.label) },
            )
        }
    }
}

// ============ Screen Routes ============

/**
 * Sealed class defining all navigation destinations.
 */
sealed class Screen(val route: String) {
    data object PlanList : Screen("plans/list")
    data object PlanCreate : Screen("plans/create")
    data object PlanEdit : Screen("plans/edit?id={planId}") {
        fun withId(planId: String) = "plans/edit?id=$planId"
    }
    data object Settings : Screen("settings")
}

/**
 * Data class for bottom navigation items.
 */
private data class BottomNavItem(
    val screen: Screen,
    val label: String,
    val icon: ImageVector,
)
